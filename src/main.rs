use std::cmp::max;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::thread::JoinHandle;
use std::{
    error::Error,
    fs::File,
    io::Read,
    time::{Duration, Instant},
};

use argh::FromArgs;
use nix::sys::socket::{GetSockOpt, SetSockOpt};

const MAX_BUFFER_BYTES: usize = 16 * 1024 * 1024;
const TARGET_TIMING: Duration = Duration::from_secs(2);

#[derive(FromArgs)]
/// buffersizebench benchmarks system calls with different buffer sizes.
struct Args {
    #[argh(option, description = "bytes for setsockopt(SO_SNDBUF)", default = "0")]
    unix_so_sndbuf: usize,

    #[argh(option, description = "only run the TCP writer", default = "0")]
    writer_only_port: u16,

    #[argh(
        option,
        description = "address to connect to for the TCP tests",
        default = "String::new()"
    )]
    writer_addr: String,

    #[argh(
        option,
        description = "write buffer size in bytes to start",
        default = "4096"
    )]
    write_buffer_bytes_start: usize,
    #[argh(
        option,
        description = "write buffer size in bytes to end (inclusive)",
        default = "16777216"
    )]
    write_buffer_bytes_end: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    assert!(args.write_buffer_bytes_start <= args.write_buffer_bytes_end);
    assert!(args.write_buffer_bytes_end <= MAX_BUFFER_BYTES);

    let mut read_buffer = vec![1; MAX_BUFFER_BYTES];

    if args.writer_only_port != 0 {
        println!(
            "Listening for TCP connections on port {} ...",
            args.writer_only_port
        );
        let listen_addr = format!("0.0.0.0:{}", args.writer_only_port);
        let tcp_listener = TcpListener::bind(listen_addr)?;
        tcp_write_acceptor(tcp_listener);
        return Ok(());
    }

    let mut maybe_tcp_acceptor_thread: Option<JoinHandle<()>> = None;
    let mut tcp_exit_fd: Option<i32> = None;
    let tcp_writer_addr = if args.writer_addr.is_empty() {
        let tcp_listener = TcpListener::bind("localhost:0")?;
        tcp_exit_fd = Some(tcp_listener.as_raw_fd());
        let tcp_writer_addr = tcp_listener.local_addr()?;
        maybe_tcp_acceptor_thread =
            Some(std::thread::spawn(move || tcp_write_acceptor(tcp_listener)));
        tcp_writer_addr
    } else {
        args.writer_addr.parse()?
    };

    let mut write_buffer_bytes = args.write_buffer_bytes_start;
    while write_buffer_bytes <= args.write_buffer_bytes_end {
        println!("\n## write_buffer_bytes={write_buffer_bytes}");

        println!("UNIX socket:");
        let (writer_sock, mut reader_sock) = UnixStream::pair()?;
        if args.unix_so_sndbuf != 0 {
            println!(
                "Calling setsockopt(SO_SNDBUF, {}) for local unix socket ...",
                args.unix_so_sndbuf
            );
            nix::sys::socket::sockopt::SndBuf.set(writer_sock.as_raw_fd(), &args.unix_so_sndbuf)?;
        }

        let writer_thread =
            std::thread::spawn(move || socket_writer(writer_sock, write_buffer_bytes));
        run_benchmark(&mut reader_sock, &mut read_buffer)?;
        // close the reader end of the socket: cause the writer to exit
        drop(reader_sock);
        writer_thread.join().expect("BUG");

        let mut reader_sock = TcpStream::connect(tcp_writer_addr)?;
        let rcvbuf = nix::sys::socket::sockopt::RcvBuf
            .get(reader_sock.as_raw_fd())
            .expect("BUG");
        // write the buffer size we want the sender to use
        let buffer_bytes_serialized = (write_buffer_bytes as u32).to_le_bytes();
        reader_sock.write_all(&buffer_bytes_serialized)?;
        println!("\nTCP writer_addr={tcp_writer_addr}; reader_sock SO_RCVBUF={rcvbuf}:",);
        run_benchmark(&mut reader_sock, &mut read_buffer)?;
        drop(reader_sock);

        println!("\n/dev/zero:");
        let mut devzero = File::open("/dev/zero")?;
        run_benchmark(&mut devzero, &mut read_buffer)?;

        println!("\n/dev/urandom:");
        let mut devurandom = File::open("/dev/urandom")?;
        run_benchmark(&mut devurandom, &mut read_buffer)?;

        write_buffer_bytes *= 2;
    }

    if let Some(writer_thread) = maybe_tcp_acceptor_thread {
        println!("waiting for TCP acceptor to shut down? Maybe we shouldn't bother ...");
        nix::sys::socket::shutdown(tcp_exit_fd.unwrap(), nix::sys::socket::Shutdown::Read)?;
        writer_thread.join().expect("BUG");
    }

    Ok(())
}

fn tcp_write_acceptor(tcp_listener: TcpListener) {
    loop {
        match tcp_listener.accept() {
            Ok((stream, _)) => {
                std::thread::spawn(move || tcp_writer(stream));
            }
            Err(err) => {
                match err.kind() {
                    std::io::ErrorKind::InvalidInput => {
                        // assume the benchmark ended correctly
                        println!("tcp acceptor: EINVAL; assuming this means we should exit");
                        break;
                    }
                    _ => {
                        panic!("tcp acceptor: unexpected socket error: {err:?}");
                    }
                }
            }
        }
    }

    // fixes clippy::needless_pass_by_value
    drop(tcp_listener);
}

fn tcp_writer(mut tcp_stream: TcpStream) {
    let sndbuf = nix::sys::socket::sockopt::SndBuf
        .get(tcp_stream.as_raw_fd())
        .expect("BUG");
    // nix::sys::socket::sockopt::SndBuf
    //     .set(stream.as_raw_fd(), &1048576)
    //     .expect("BUG");

    let mut buffer_bytes_serialized = [0u8; 4];
    tcp_stream
        .read_exact(&mut buffer_bytes_serialized)
        .expect("BUG");
    let write_buffer_bytes = u32::from_le_bytes(buffer_bytes_serialized);

    println!("TCP writer stared; SO_SNDBUF={sndbuf}; write_buffer_bytes={write_buffer_bytes}");

    socket_writer(tcp_stream, write_buffer_bytes as usize);
}

fn socket_writer<W: Write>(mut sock: W, write_buffer_bytes: usize) {
    assert!(write_buffer_bytes <= MAX_BUFFER_BYTES);
    let buffer = vec![0; write_buffer_bytes];
    loop {
        match sock.write(&buffer) {
            Ok(num_bytes) => {
                // writes never seem to return partial blocks except when the reader is closed
                // we get these partial writes when writing with huge buffers; with small buffers
                // this doesn't seem to happen
                if num_bytes < buffer.len() {
                    println!("short write {num_bytes} total bytes; {} bytes short; expecting EPIPE on next write ...",
                        buffer.len() - num_bytes);
                    match sock.write(&buffer) {
                        Ok(_) => {
                            panic!("unexpected")
                        }

                        Err(err) => {
                            match err.kind() {
                                std::io::ErrorKind::ConnectionReset => {
                                    // assume the benchmark ended correctly
                                    println!("ECONNRESET at end of benchmark after partial write");
                                    return;
                                }
                                std::io::ErrorKind::BrokenPipe => {
                                    // assume the benchmark ended correctly
                                    println!("EPIPE at end of benchmark after partial write");
                                    return;
                                }
                                _ => {
                                    panic!("unexpected unix socket error: {err:?}");
                                }
                            }
                        }
                    }
                }
            }
            Err(err) => {
                match err.kind() {
                    std::io::ErrorKind::ConnectionReset => {
                        // assume the benchmark ended correctly
                        println!("ECONNRESET at end of benchmark");
                        return;
                    }
                    std::io::ErrorKind::BrokenPipe => {
                        // assume the benchmark ended correctly
                        println!("EPIPE at end of benchmark");
                        return;
                    }
                    _ => {
                        panic!("unexpected unix socket error: {err:?}");
                    }
                }
            }
        }
    }
}

fn run_benchmark<R: Read>(f: &mut R, buffer: &mut [u8]) -> Result<(), std::io::Error> {
    // run a first timing loop to "warm up" everything: the first calls to a unix socket are slow
    _ = time_reads(f, &mut buffer[0..1024], 1024 * 4)?;

    let mut buf_size = 1;
    let mut highest_mib_per_sec = 0.0;
    let mut highest_mib_per_sec_buffer = 0;
    let mut throughput_increasing = true;
    while buf_size <= MAX_BUFFER_BYTES {
        const TIMING_ESTIMATE_SYSCALLS: usize = 100;
        let sized_buf: &mut [u8] = &mut buffer[0..buf_size];
        let estimate_results = time_reads(f, sized_buf, TIMING_ESTIMATE_SYSCALLS * buf_size)?;
        let target_total_bytes =
            ((TARGET_TIMING.as_secs_f64() / estimate_results.duration.as_secs_f64())
                * (TIMING_ESTIMATE_SYSCALLS * buf_size) as f64) as usize;
        let measure_bytes = max(target_total_bytes, TIMING_ESTIMATE_SYSCALLS);
        // println!(
        //     "buf_size={buf_size} estimate_duration={:?} measure_bytes={measure_bytes}",
        //     estimate_results.duration
        // );

        let results = time_reads(f, sized_buf, measure_bytes)?;
        let mib_per_sec =
            (results.total_bytes) as f64 / 1024. / 1024. / results.duration.as_secs_f64();
        let syscalls_per_sec = results.num_syscalls as f64 / results.duration.as_secs_f64();
        println!("buf_size={buf_size}; duration={:?}; num_syscalls={}; {mib_per_sec:.1} MiB/s; {syscalls_per_sec:.1} syscalls/s; short_reads={}",
            results.duration, results.num_syscalls, results.short_reads);

        if highest_mib_per_sec < mib_per_sec {
            if throughput_increasing {
                highest_mib_per_sec = mib_per_sec;
                highest_mib_per_sec_buffer = buf_size;
            }
        } else {
            // throughput went down!
            throughput_increasing = false;
        }

        buf_size *= 2;
    }
    println!("  BEST: buf_size={highest_mib_per_sec_buffer} {highest_mib_per_sec:.1} MiB/s");
    Ok(())
}

struct RunStats {
    total_bytes: usize,
    num_syscalls: usize,
    short_reads: usize,
    duration: Duration,
}

fn time_reads<R: Read>(
    f: &mut R,
    buffer: &mut [u8],
    total_bytes: usize,
) -> Result<RunStats, std::io::Error> {
    let mut total_bytes_read = 0;
    let mut num_syscalls = 0;
    let mut short_reads = 0;
    let start = Instant::now();
    while total_bytes_read < total_bytes {
        let bytes_read = f.read(buffer)?;
        assert!(0 < bytes_read && bytes_read <= buffer.len());
        if bytes_read < buffer.len() {
            short_reads += 1;
        }
        total_bytes_read += bytes_read;
        num_syscalls += 1;
    }
    let end = Instant::now();

    let duration = end - start;
    Ok(RunStats {
        total_bytes,
        num_syscalls,
        short_reads,
        duration,
    })
}
