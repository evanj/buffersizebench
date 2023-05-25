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

    #[argh(option, description = "bytes ", default = "16777216")]
    writer_buffer_bytes: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    assert!(args.writer_buffer_bytes <= MAX_BUFFER_BYTES);

    let mut read_buffer = vec![1; MAX_BUFFER_BYTES];

    if args.writer_only_port != 0 {
        println!(
            "Listening for TCP connections on port {} ...",
            args.writer_only_port
        );
        let listen_addr = format!("0.0.0.0:{}", args.writer_only_port);
        let tcp_listener = TcpListener::bind(listen_addr)?;
        tcp_writer(tcp_listener, args.writer_buffer_bytes);
        return Ok(());
    }

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
        std::thread::spawn(move || socket_writer(writer_sock, args.writer_buffer_bytes));
    run_benchmark(&mut reader_sock, &mut read_buffer)?;
    // close the reader end of the socket: cause the writer to exit
    drop(reader_sock);
    writer_thread.join().expect("BUG");

    let mut maybe_writer_thread: Option<JoinHandle<()>> = None;
    let writer_addr = if args.writer_addr.is_empty() {
        let tcp_listener = TcpListener::bind("localhost:0")?;
        let writer_addr = tcp_listener.local_addr()?;
        maybe_writer_thread = Some(std::thread::spawn(move || {
            tcp_writer(tcp_listener, args.writer_buffer_bytes)
        }));
        writer_addr
    } else {
        args.writer_addr.parse()?
    };

    let mut reader_sock = TcpStream::connect(writer_addr)?;
    let rcvbuf = nix::sys::socket::sockopt::RcvBuf
        .get(reader_sock.as_raw_fd())
        .expect("BUG");
    println!("\nTCP writer_addr={writer_addr}; reader_sock SO_RCVBUF={rcvbuf}:",);
    // nix::sys::socket::sockopt::RcvBuf
    //     .set(reader_sock.as_raw_fd(), &(1048576 * 8))
    //     .expect("BUG");
    println!("TCP reader sock; SO_RCVBUF={rcvbuf}");
    run_benchmark(&mut reader_sock, &mut read_buffer)?;
    drop(reader_sock);
    if let Some(writer_thread) = maybe_writer_thread {
        writer_thread.join().expect("BUG");
    }

    println!("\n/dev/zero:");
    let mut devzero = File::open("/dev/zero")?;
    run_benchmark(&mut devzero, &mut read_buffer)?;

    println!("\n/dev/urandom:");
    let mut devurandom = File::open("/dev/urandom")?;
    run_benchmark(&mut devurandom, &mut read_buffer)?;

    Ok(())
}

fn tcp_writer(tcp_listener: TcpListener, writer_buffer_bytes: usize) {
    let (stream, _) = tcp_listener.accept().expect("tcp_writer BUG");

    let sndbuf = nix::sys::socket::sockopt::SndBuf
        .get(stream.as_raw_fd())
        .expect("BUG");
    println!("TCP accepted new connection; writer SO_SNDBUF={sndbuf}");
    // nix::sys::socket::sockopt::SndBuf
    //     .set(stream.as_raw_fd(), &1048576)
    //     .expect("BUG");

    socket_writer(stream, writer_buffer_bytes);
    // avoids clippy::needless-pass-by-value warning
    drop(tcp_listener);
}

fn socket_writer<W: Write>(mut sock: W, writer_buffer_bytes: usize) {
    let buffer = vec![0; writer_buffer_bytes];
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

        buf_size *= 2;
    }
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
