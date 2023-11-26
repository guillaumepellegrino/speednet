use eyre::{eyre, Result, WrapErr};
use std::net::{TcpStream, UdpSocket, SocketAddr, IpAddr};
use crate::{
    args::ArgsClient,
    message::{Message, MessageIO},
    pktgenerator,
};

pub struct Client {
    args: ArgsClient,
    control_addr: SocketAddr,
    control_stream: TcpStream,
}

struct Stream {
    args: ArgsClient,
    testid: u32,
    control_addr: SocketAddr,
}

impl Stream {
    pub fn new(client: &Client, testid: u32) -> Self {
        Self {
            args: client.args.clone(),
            testid,
            control_addr: client.control_addr.clone(),
        }
    }

    pub fn run(&self) -> Result<()> {
        if self.args.udp {
            self.run_udp()?;
        }
        else {
            self.run_tcp()?;
        }
        Ok(())
    }

    pub fn run_udp(&self) -> Result<()> {
        let bindaddr = match self.control_addr.is_ipv4() {
            true  => "0.0.0.0:0",
            false => "[::0]:0",
        };
        let mut s = UdpSocket::bind(bindaddr)
            .wrap_err("Failed to bind addr")?;

        let start_udp = Message::ClientStartUDP(self.testid);
        s.sendmsg(&start_udp)
            .wrap_err("Client failed to start UDP")?;


        Ok(())
    }

    pub fn run_tcp(&self) -> Result<()> {
        let mut stream = TcpStream::connect(self.control_addr)
            .wrap_err("Failed to connect to server")?;

        let start_stream = Message::ClientStartStream(self.testid);
        stream.sendmsg(&start_stream)
            .wrap_err("Client failed to start stream")?;

        if self.args.revert {
            self.run_tcp_download(stream)?;
        }
        else {
            self.run_tcp_upload(stream)?;
        }

        Ok(())
    }

    pub fn run_tcp_upload(&self, stream: TcpStream) -> Result<()> {
        println!("TCP Upload");
        let result = pktgenerator::tcp_send(&self.args, stream, |update| {
            println!("Elapsed: {}", update.elapsed.as_secs());
            println!("pktsent: {}", update.pktcount);
            println!("expected: {}", update.pktcount_expected);
            println!("");
        })?;
        println!("TCP Upload done");
        println!("Elapsed: {}", result.elapsed.as_secs());
        println!("pktsent: {}", result.pktcount);
        Ok(())
    }

    pub fn run_tcp_download(&self, stream: TcpStream) -> Result<()> {
        println!("TCP Download");
        let result = pktgenerator::tcp_recv(&self.args, stream, |update| {
            println!("Elapsed: {}", update.elapsed.as_secs());
            println!("pktrecv: {}", update.pktcount);
            println!("");
        })?;
        println!("TCP Download done");
        println!("Elapsed: {}", result.elapsed.as_secs());
        println!("Pkt Recv: {}", result.pktcount);
        Ok(())
    }
}

impl Client {
    pub fn new(args: ArgsClient) -> Result<Self> {
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
        })
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
    /// - [ctl] Server report stats every second and when conn is closed
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
    /// - [ctl] Server report stats every second and when conn is closed
    ///
    pub fn run(&mut self) -> Result<()> {
        let client_hello = Message::ClientHello(self.args.clone());
        self.control_stream.sendmsg(&client_hello)
            .wrap_err("Failed to send client hello to server")?;

        let msg = self.control_stream.recvmsg()
            .wrap_err("Failed to read server hello message")?;

        let testid = match msg {
            Message::ServerHello(testid) => testid,
            _ => {return Err(eyre!("Expected ServerHello message iso {:?}", msg));},
        };

        let mut threads = vec!();
        for _ in 0 .. self.args.parallel {
            let stream = Stream::new(self, testid);
            let thread = std::thread::spawn(move || {
                if let Err(e) = stream.run() {
                    println!("Failed to run stream: {:?}", e);
                }
            });
            threads.push(thread);
        }

        for thread in threads {
            if let Err(e) = thread.join() {
                println!("Thead returned an error: {:?}", e);
            }
        }

        Ok(())
    }
}
