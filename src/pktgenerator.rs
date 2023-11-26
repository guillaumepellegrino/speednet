use eyre::{Result, WrapErr};
use std::time::{Instant, Duration};
use std::thread::sleep;
use std::net::{TcpStream};
use std::io::{Read, Write};
use crate::args::ArgsClient;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Update {
    pub elapsed: Duration,
    pub pktcount_expected: u128,
    pub pktcount: u128,
}

pub fn tcp_send<F: FnMut(&Update)>(args: &ArgsClient, mut stream: TcpStream, mut update_cb: F) -> Result<Update> {
    let bufferlen = args.get_bufferlen();
    let mut buffer = Vec::with_capacity(bufferlen as usize);
    for i in 1..bufferlen {
        let value : u64 = i % 255;
        buffer.push(value as u8);
    }
    let duration = Duration::from_secs(args.time);
    let bandwidth = args.get_bandwidth();
    let total_packets = args.get_totalpackets();
    let now = Instant::now();

    println!("Duration: {}", args.time);
    println!("Bandwidth: {}", bandwidth);
    println!("Bufferlen: {}", bufferlen);
    println!("Total packets: {}", total_packets);
    println!("");

    let mut update = Update::default();
    let mut prev_elapsed = Duration::from_secs(0);
    loop {
        update.elapsed = now.elapsed();
        update.pktcount_expected = (total_packets as u128 * update.elapsed.as_nanos()) / duration.as_nanos();
        if update.elapsed.as_secs() != prev_elapsed.as_secs() {
            update_cb(&update);
            prev_elapsed = update.elapsed;
        }
        if update.elapsed.as_secs() >= args.time {
            break;
        }
        if (bandwidth > 0) && (update.pktcount >= update.pktcount_expected) {
            sleep(Duration::from_millis(1));
            continue;
        }

        let len = stream.write(&mut buffer)?;
        if len == 0 {
            println!("Connection to server closed");
            break;
        }
        update.pktcount += 1;
    }
    Ok(update)
}

pub fn tcp_recv<F: FnMut(&Update)>(args: &ArgsClient, mut stream: TcpStream, mut update_cb: F) -> Result<Update> {
    let mut update = Update::default();
    let bufferlen = args.get_bufferlen();
    let mut buffer = vec!(0; bufferlen as usize);
    let now = Instant::now();

    let mut prev_elapsed = Duration::from_secs(0);
    loop {
        update.elapsed = now.elapsed();
        if update.elapsed.as_secs() != prev_elapsed.as_secs() {
            update_cb(&update);
            prev_elapsed = update.elapsed;
        }

        let len = stream.read(&mut buffer)
            .wrap_err("Failed to read")?;

        if len == 0 {
            break;
        }
        update.pktcount += 1;
    }

    Ok(update)
}

