use std::time::{Instant, Duration};
use std::thread::sleep;
use std::net::{TcpStream};
use std::io::{Read, Write};
use std::sync::Mutex;
use crate::args::ArgsClient;
use crate::result::StreamResult;

pub fn tcp_send(args: &ArgsClient, mut stream: TcpStream, update: &Mutex<StreamResult>) {
    let bufferlen = args.get_bufferlen();
    let mut buffer = Vec::with_capacity(bufferlen as usize);
    for i in 1..bufferlen {
        let value : u64 = i % 255;
        buffer.push(value as u8);
    }
    let duration = Duration::from_secs(args.time);
    let total_packets = args.get_totalpackets();
    let mut pktcount = 0;
    let mut bytes = 0;
    let now = Instant::now();
    let mut pktcount_expected = 0;
    let mut elapsed;

    loop {
        if pktcount >= pktcount_expected {
            sleep(Duration::from_millis(2));
            elapsed = now.elapsed();
            pktcount_expected = if elapsed.as_secs() >= args.time {
                total_packets
            }
            else {
                ((total_packets as u128 * (elapsed.as_nanos())) / duration.as_nanos()) as u64
            };    
            continue;
        }

        let len = match stream.write(&mut buffer) {
            Ok(x) => x,
            Err(_) => break,
        };
        if len == 0 {
            println!("Connection to server closed");
            break;
        }
        pktcount += 1;
        bytes += len as u64;
        elapsed = now.elapsed();
        pktcount_expected = if elapsed.as_secs() >= args.time {
            total_packets
        }
        else {
            ((total_packets as u128 * (elapsed.as_nanos())) / duration.as_nanos()) as u64
        };

        let mut update = update.lock().unwrap();
        update.pktcount = pktcount;
        update.pktcount_expected = pktcount_expected;
        update.bytes = bytes;
        update.elapsed = elapsed;

        if elapsed.as_secs() >= args.time {
            if pktcount >= pktcount_expected {
                update.testdone = true;
                break;
            }
            if elapsed.as_secs() >= args.time + 1 {
                update.testdone = true;
                break;
            }
        }
    }
}

pub fn tcp_recv(args: &ArgsClient, mut stream: TcpStream, update: &Mutex<StreamResult>) {
    let bufferlen = args.get_bufferlen();
    let mut buffer = vec!(0; bufferlen as usize);
    let total_packets = args.get_totalpackets();
    let mut pktcount = 0;
    let mut bytes = 0;
    let now = Instant::now();

    loop {
        // TODO: use alarm(1) to protect thread from being stuck in read syscall.
        let len = match stream.read(&mut buffer) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to read: {:?}", e);
                break;
            }
        };
        if len == 0 {
            break;
        }

        pktcount += 1;
        bytes += len as u64;
        let elapsed = now.elapsed();

        let mut update = update.lock().unwrap();
        update.pktcount = pktcount;
        update.bytes = bytes;
        update.elapsed = now.elapsed();
        update.pktcount_expected = total_packets;
        if elapsed.as_secs() >= args.time {
            if pktcount >= total_packets {
                update.testdone = true;
                break;
            }
            if elapsed.as_secs() >= args.time + 1 {
                update.testdone = true;
                break;
            }
        }
    }
}