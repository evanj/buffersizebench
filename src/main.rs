use std::cmp::max;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::UnixStream;
use std::{
    error::Error,
    fs::File,
    io::Read,
    time::{Duration, Instant},
};

const MAX_BUF_SIZE: usize = 16 * 1024 * 1024;
const TARGET_TIMING: Duration = Duration::from_secs(2);

fn main() -> Result<(), Box<dyn Error>> {
    let mut buffer = vec![1; MAX_BUF_SIZE];

    println!("UNIX socket:");
    let (writer_sock, mut reader_sock) = UnixStream::pair()?;
    let writer_thread = std::thread::spawn(|| socket_writer(writer_sock));
    run_benchmark(&mut reader_sock, &mut buffer)?;
    // close the reader end of the socket: cause the writer to exit
    drop(reader_sock);
    writer_thread.join().expect("BUG");

    println!("Localhost TCP:");
    let tcp_listener = TcpListener::bind("localhost:0")?;
    let listener_addr = tcp_listener.local_addr()?;
    let writer_thread = std::thread::spawn(|| tcp_writer(tcp_listener));
    let mut reader_sock = TcpStream::connect(listener_addr)?;
    run_benchmark(&mut reader_sock, &mut buffer)?;
    drop(reader_sock);
    writer_thread.join().expect("BUG");

    println!("\n/dev/zero:");
    let mut devzero = File::open("/dev/zero")?;
    run_benchmark(&mut devzero, &mut buffer)?;

    let mut devurandom = File::open("/dev/urandom")?;
    run_benchmark(&mut devurandom, &mut buffer)?;

    Ok(())
}

fn tcp_writer(tcp_listener: TcpListener) {
    let (stream, _) = tcp_listener.accept().expect("tcp_writer BUG");
    println!("accepted new connection");
    socket_writer(stream);
    // avoids clippy::needless-pass-by-value warning
    drop(tcp_listener);
}

fn socket_writer<W: Write>(mut sock: W) {
    let buffer = vec![0; MAX_BUF_SIZE];
    loop {
        match sock.write(&buffer) {
            Ok(num_bytes) => {
                // writes never seem to return partial blocks except when the reader is closed
                // we get these partial writes when writing with huge buffers; with small buffers
                // this doesn't seem to happen
                if num_bytes < buffer.len() {
                    println!("short write {num_bytes} total bytes; {} bytes short; expecting EPIPE on next write ...",
                        buffer.len() - num_bytes);
                    match sock.write(&buffer[..MAX_BUF_SIZE]) {
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
    while buf_size <= MAX_BUF_SIZE {
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
