use std::cmp::max;
use std::{
    error::Error,
    fs::File,
    io::Read,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn Error>> {
    const MAX_BUF_SIZE: usize = 16 * 1024 * 1024;
    const TARGET_TIMING: Duration = Duration::from_secs(2);
    let mut buffer = vec![1; MAX_BUF_SIZE];

    // let mut devzero = File::open("/dev/urandom")?;
    let mut devzero = File::open("/dev/zero")?;

    let mut buf_size = 1;
    while buf_size <= MAX_BUF_SIZE {
        const TIMING_ESTIMATE_SYSCALLS: usize = 100;
        let sized_buf: &mut [u8] = &mut buffer[0..buf_size];
        let estimate_duration = time_reads(&mut devzero, sized_buf, TIMING_ESTIMATE_SYSCALLS)?;
        let target_num_syscalls = ((TARGET_TIMING.as_secs_f64() / estimate_duration.as_secs_f64())
            * TIMING_ESTIMATE_SYSCALLS as f64) as usize;
        let num_syscalls = max(target_num_syscalls, TIMING_ESTIMATE_SYSCALLS);
        // println!(
        //     "buf_size={buf_size} estimate_duration={estimate_duration:?} num_syscalls={num_syscalls}"
        // );

        let duration = time_reads(&mut devzero, sized_buf, num_syscalls)?;
        let mib_per_sec =
            (num_syscalls * sized_buf.len()) as f64 / 1024. / 1024. / duration.as_secs_f64();
        let syscalls_per_sec = num_syscalls as f64 / duration.as_secs_f64();
        println!("buf_size={buf_size}; duration={duration:?}; num_syscalls={num_syscalls}; {mib_per_sec:.1} MiB/s; {syscalls_per_sec:.1} syscalls/s");

        buf_size *= 2;
    }

    Ok(())
}

fn time_reads(
    f: &mut File,
    buffer: &mut [u8],
    num_calls: usize,
) -> Result<Duration, std::io::Error> {
    let start = Instant::now();
    for _ in 0..num_calls {
        let bytes_read = f.read(buffer)?;
        assert!(
            bytes_read == buffer.len(),
            "bytes_read={bytes_read}; buffer.len()={}",
            buffer.len()
        );
    }
    let end = Instant::now();

    Ok(end - start)
}
