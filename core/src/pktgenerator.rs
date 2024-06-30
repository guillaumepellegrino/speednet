use std::time::{Instant, Duration};
use std::thread::sleep;
use std::net::{TcpStream};
use std::io::{Read, Write};
use std::sync::Mutex;
use crate::args::ArgsClient;
use crate::result::StreamResult;

pub fn tcp_send(args: &ArgsClient, mut stream: TcpStream) {
    let bufferlen = args.get_bufferlen();
    let mut buffer = Vec::with_capacity(bufferlen as usize);
    for i in 1..bufferlen {
        let value : u64 = i % 255;
        buffer.push(value as u8);
    }
    let duration = Duration::from_secs(args.time);
    let total_bytes_expected = args.get_total_bytes_expected();
    let mut bytes = 0;
    let now = Instant::now();
    let mut bytes_expected = 0;
    let mut elapsed;

    if let Err(e) = stream.set_write_timeout(Some(Duration::from_millis(1000))) {
        println!("Failed to set write timeout: {:?}", e);
        return;
    }

    loop {
        if bytes >= bytes_expected {
            sleep(Duration::from_millis(2));
            elapsed = now.elapsed();
            bytes_expected = if elapsed.as_secs() >= args.time {
                total_bytes_expected
            }
            else {
                ((total_bytes_expected as u128 * (elapsed.as_nanos())) / duration.as_nanos()) as u64
            };    
            continue;
        }

        let remaining_bytes = total_bytes_expected - bytes;
        let len = std::cmp::min(remaining_bytes, bufferlen-1) as usize;
        let buffer = &mut buffer[0..len];
        let len = match stream.write(buffer) {
            Ok(x) => x,
            Err(e) => {
                println!("[PKTGEN.TX] Write error: {:?}", e);
                break;
            },
        };
        if len == 0 {
            println!("[PKTGEN.TX] Connection closed by remote peer");
            break;
        }
        bytes += len as u64;
        elapsed = now.elapsed();
        bytes_expected = if elapsed.as_secs() >= args.time {
            total_bytes_expected
        }
        else {
            ((total_bytes_expected as u128 * (elapsed.as_nanos())) / duration.as_nanos()) as u64
        };
        if bytes >= total_bytes_expected {
            //println!("[PKTGEN.TX] bytes={bytes}, total_bytes_expected={total_bytes_expected}");
            break;
        }
        if elapsed.as_secs() >= args.time + 1 {
            println!("[PKTGEN.TX] Timeout: bytes={bytes}, total_bytes_expected={total_bytes_expected}");
            break;
        }
    }
}

pub fn tcp_recv(args: &ArgsClient, mut stream: TcpStream, update: &Mutex<StreamResult>) {
    let bufferlen = args.get_bufferlen();
    let mut buffer = vec!(0; bufferlen as usize);
    let total_bytes_expected = args.get_total_bytes_expected();
    let mut pktcount = 0;
    let mut bytes = 0;
    let now = Instant::now();

    if let Err(e) = stream.set_read_timeout(Some(Duration::from_millis(1000))) {
        println!("Failed to set read timeout: {:?}", e);
        return;
    }

    loop {
        // TODO: use alarm(1) to protect thread from being stuck in read syscall.
        //       or maybe a non-blocking read syscall + select ?
        //
        //       if read(s) == Err(EAGAIN) {
        //          select(s)
        //          read(s)
        //       }
        let len = match stream.read(&mut buffer) {
            Ok(x) => x,
            Err(e) => {
                println!("[PKTGEN.RX] Failed to read: {:?}", e);
                let mut update = update.lock().unwrap();
                update.testdone = true;    
                break;
            }
        };
        if len == 0 {
            println!("[PKTGEN.RX] Connection closed by remote peer");
            println!("[PKTGEN.RX] bytes: {bytes}, total_bytes_expected: {total_bytes_expected}");
            let mut update = update.lock().unwrap();
            update.testdone = true;
            break;
        }

        let elapsed = now.elapsed();
        pktcount += 1;
        bytes += len as u64;

        let mut update = update.lock().unwrap();
        update.pktcount = pktcount;
        update.bytes = bytes;
        update.elapsed = elapsed;
        update.bytes_expected = total_bytes_expected;


        if bytes >= total_bytes_expected {
            //println!("[PKTGEN.RX] bytes: {bytes}, total_bytes_expected: {total_bytes_expected}");
            update.testdone = true;
            break;
        }
        if elapsed.as_secs() >= args.time + 1 {
            println!("[PKTGEN.RX] Timeout. bytes: {bytes}, total_bytes_expected: {total_bytes_expected}");
            update.testdone = true;
            break;
        }
    }
}
