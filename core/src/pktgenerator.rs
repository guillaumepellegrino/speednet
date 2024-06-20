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

    loop {
        let elapsed = now.elapsed();
        let pktcount_expected;

        if elapsed.as_secs() >= args.time {
            pktcount_expected = total_packets;
            if pktcount >= pktcount_expected {
                break;
            }
            if elapsed.as_secs() >= args.time + 1 {
                break;
            }
        }
        else {
            pktcount_expected = ((total_packets as u128 * (elapsed.as_nanos())) / duration.as_nanos()) as u64;
        }
        if pktcount >= pktcount_expected {
            sleep(Duration::from_millis(2));
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

        let mut update = update.lock().unwrap();
        update.pktcount = pktcount;
        update.bytes = bytes;
    }

    let mut update = update.lock().unwrap();
    update.elapsed = now.elapsed();
    update.pktcount_expected = total_packets;
}

pub fn tcp_recv(args: &ArgsClient, mut stream: TcpStream, update: &Mutex<StreamResult>) {
    let bufferlen = args.get_bufferlen();
    let mut buffer = vec!(0; bufferlen as usize);
    let total_packets = args.get_totalpackets();
    let mut pktcount = 0;
    let mut bytes = 0;
    let now = Instant::now();


    loop {
        let elapsed = now.elapsed();
        if elapsed.as_secs() >= args.time {
            if pktcount >= total_packets {
                break;
            }
            if elapsed.as_secs() >= args.time + 1 {
                break;
            }
        }
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

        let mut update = update.lock().unwrap();
        update.pktcount = pktcount;
        update.bytes = bytes;    
    }

    let mut update = update.lock().unwrap();
    update.elapsed = now.elapsed();
    update.pktcount_expected = total_packets;
}