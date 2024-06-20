use eyre::{eyre, Result, WrapErr};
use std::net::{TcpStream, UdpSocket, SocketAddr, IpAddr};
use std::sync::{Mutex};
use std::time::{Duration, Instant};
use crate::{
    args::ArgsClient,
    api::{Message, MessageIO},
    pktgenerator,
    result::{StreamResult, ClientResult},
};

pub struct Client<'a> {
    args: ArgsClient,
    control_addr: SocketAddr,
    control_stream: TcpStream,
    update_cb: Option<Box<dyn FnMut(&ClientResult) + Send + 'a>>
}

struct Stream {
    args: ArgsClient,
    testid: u32,
    streamid: u32,
    control_addr: SocketAddr,
}

impl Stream {
    fn new(client: &Client, testid: u32, streamid: u32) -> Self {
        Self {
            args: client.args.clone(),
            testid,
            streamid,
            control_addr: client.control_addr.clone(),
        }
    }

    fn run(&mut self, result: &Mutex<StreamResult>) -> Result<()> {
        if self.args.udp {
            self.run_udp()?;
        }
        else {
            self.run_tcp(result)?;
        }
        Ok(())
    }

    fn run_udp(&mut self) -> Result<StreamResult> {
        let bindaddr = match self.control_addr.is_ipv4() {
            true  => "0.0.0.0:0",
            false => "[::0]:0",
        };
        let mut s = UdpSocket::bind(bindaddr)
            .wrap_err("Failed to bind addr")?;

        let start_udp = Message::ClientStreamHello(self.testid, self.streamid);
        s.sendmsg(&start_udp)
            .wrap_err("Client failed to start UDP")?;

        unreachable!();
    }

    fn run_tcp(&mut self, result: &Mutex<StreamResult>) -> Result<()> {
        let mut stream = TcpStream::connect(self.control_addr)
            .wrap_err("Failed to connect to server")?;

        let start_stream = Message::ClientStreamHello(self.testid, self.streamid);
        stream.sendmsg(&start_stream)
            .wrap_err("Client failed to start stream")?;

        if self.args.revert {
            pktgenerator::tcp_recv(&self.args, stream, result);
        }
        else {
            pktgenerator::tcp_send(&self.args, stream, result);
        }
        Ok(())
    }
}

impl<'a> Client<'a> {
    pub fn new(mut args: ArgsClient) -> Result<Self> {
        args.prepare_config();
        args.print_config();

        let ip_addr = args.hostname.parse::<IpAddr>()
            .wrap_err("Invalid hostname")?;

        let addr = SocketAddr::new(ip_addr, args.port);
        println!("speednet client connect to {:?}", addr);

        let stream = TcpStream::connect(addr)
            .wrap_err("Failed to connect to server")?;

        Ok(Self {
            args,
            control_addr: addr,
            control_stream: stream,
            update_cb: None,
        })
    }

    pub fn on_update<F>(&mut self, update_cb: F)
        where F: FnMut(&ClientResult) + Send + 'a
    {
        self.update_cb = Some(Box::new(update_cb));
    }

    pub fn args(&self) -> &ArgsClient {
        &self.args
    }

    /// 1. TCP Upload
    /// - [ctl] Client send config to Server
    /// - [ctl] Server acknowledge
    /// - [data] Client open Nx data TCP streams
    /// - [data] Client send on data TCP stream
    /// - [ctl] Server report stats every second and when conn is closed
    ///
    /// 2. TCP Download
    /// - [ctl] Client send config to Server
    /// - [ctl] Server acknowledge
    /// - [data] Client open Nx data TCP streams
    /// - [data] Server send on data TCP stream
    ///
    /// 3. UDP Upload
    /// - [ctl] Client send config to Server
    /// - [ctl] Server acknowledge
    /// - [data] Client open Nx data UDP streams
    /// - [data] Client send on data UDP stream
    /// - [ctl] Server report stats every second and when conn is closed
    ///
    /// 4. UDP Download
    /// - [ctl] Client send config to Server
    /// - [ctl] Server acknowledge
    /// - [data] Client open Nx data UDP streams
    /// - [data] Server send on data UDP stream
    ///
    pub fn run(&mut self) -> Result<ClientResult> {
        let client_hello = Message::ClientHello(self.args.clone());
        self.control_stream.sendmsg(&client_hello)
            .wrap_err("Failed to send client hello to server")?;

        let msg = self.control_stream.recvmsg()
            .wrap_err("Failed to read server hello message")?;

        let testid = match msg {
            Message::ServerHello(testid) => testid,
            _ => {return Err(eyre!("Expected ServerHello message iso {:?}", msg));},
        };


        let mut client_result = ClientResult::default();
        let mut stream_results = vec!();
        for _streamid in 0 .. self.args.parallel {
            let stream_result = std::sync::Mutex::new(StreamResult::default());
            stream_results.push(stream_result);
        }

        std::thread::scope(|scope| {
            // start clients threads
            let mut threads = vec!();
            let stream_results = &stream_results;
            for streamid in 0 .. self.args.parallel {
                let mut stream = Stream::new(self, testid, streamid);
                let thread = scope.spawn(move || {
                    stream.run(&stream_results[streamid as usize])
                });
                threads.push(thread);
            }

            // Collect stream results every second until time is elapsed
            let total_packets = self.args.get_totalpackets();
            let duration = Duration::from_secs(self.args.time);
            let now = Instant::now();
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let elapsed = now.elapsed();
                if elapsed.as_secs() >= self.args.time {
                    break;
                }
                client_result.reset();
                for streamid in 0 .. self.args.parallel {
                    let mut stream_result = stream_results[streamid as usize]
                        .lock().unwrap().clone();
                    stream_result.elapsed = elapsed;
                    stream_result.pktcount_expected = ((total_packets as u128 * elapsed.as_nanos()) / duration.as_nanos()) as u64;
                    client_result.add_stream_result(&stream_result);
                }
                if let Some(update_cb) = &mut self.update_cb {
                    update_cb(&client_result);
                }
            }
        });

        // Build final result
        client_result.reset();
        for streamid in 0 .. self.args.parallel {
            let stream_result = stream_results[streamid as usize]
                .lock().unwrap().clone();
            client_result.add_stream_result(&stream_result);
        }

        Ok(client_result)
    }
}
