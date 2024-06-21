use eyre::{eyre, Result, WrapErr};
use std::net::{TcpStream, UdpSocket, SocketAddr, IpAddr};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};
use crate::{
    args::ArgsClient,
    api::{Message, MessageIO},
    pktgenerator,
    result::{StreamResult, ClientResult},
};

pub struct Client<'a> {
    args: ArgsClient, // client configuration passed by command-line arguments
    control_addr: SocketAddr, // socket address used to talk with server
    control_stream: TcpStream, // socket used to talk with server
    run_barrier: Arc<Barrier>, // wait for all threads to be setup before running test
    stream_results: Arc<Vec<Mutex<StreamResult>>>, // results updated by stream threads
    stream_threads: Vec<std::thread::JoinHandle<Result<()>>>, // threads running a stream download or upload
    update_cb: Option<Box<dyn FnMut(&ClientResult) + Send + 'a>> // called when an client result update is available
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
            pktgenerator::tcp_send(&self.args, stream);
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

        let run_barrier = Arc::new(Barrier::new(args.parallel as usize + 1));
        
        let mut stream_results = vec!();
        for _streamid in 0 .. args.parallel {
            let stream_result = std::sync::Mutex::new(StreamResult::default());
            stream_results.push(stream_result);
        }

        let mut me = Self {
            args,
            control_addr: addr,
            control_stream: stream,
            run_barrier,
            stream_results: Arc::new(stream_results),
            stream_threads: vec!(),
            update_cb: None,
        };
        me.setup()
            .wrap_err("Failed to setup client")?;

        Ok(me)
    }

    fn setup(&mut self) -> Result<()> {
        let client_hello = Message::ClientHello(self.args.clone());
        let testid = match self.control_stream.sendrecvmsg(&client_hello) {
            Ok(Message::ServerHello(testid)) => testid,
            Ok(other) => {return Err(eyre!("Expected ServerHello message iso {:?}", other));}
            Err(e) => {return Err(eyre!("Failed to communicate with server: {:?}", e));}
        };

        for streamid in 0 .. self.args.parallel {
            let mut stream = Stream::new(self, testid, streamid);
            let stream_results_clone = self.stream_results.clone();
            let run_barrier = self.run_barrier.clone();
            let thread = std::thread::spawn(move || {
                let stream_result = stream_results_clone.get(streamid as usize).unwrap();
                run_barrier.wait();
                stream.run(stream_result)
            });
            self.stream_threads.push(thread);
        }
        Ok(())
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
    

    /// Results are retrieved from local threads
    fn run_download(&mut self) -> Result<ClientResult> {
        // Collect stream results every second until time is elapsed
        let total_bytes_expected = self.args.get_total_bytes_expected();
        let duration = Duration::from_secs(self.args.time);
        let now = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let elapsed = now.elapsed();
            if elapsed.as_secs() >= self.args.time {
                break;
            }
            let bytes_expected = ((total_bytes_expected as u128 * (elapsed.as_nanos())) / duration.as_nanos()) as u64;
            let client_result = ClientResult::collect_and_override(&self.stream_results, elapsed, bytes_expected);
            if let Some(update_cb) = &mut self.update_cb {
                update_cb(&client_result);
            }
        }

        while let Some(thread) = self.stream_threads.pop() {
            thread.join().unwrap().unwrap();
        }

        // Build final result
        Ok(ClientResult::collect(&self.stream_results))
    }

    /// Results are retrieved from remote server
    fn run_upload(&mut self) -> Result<ClientResult> {
        loop {
            let update = match self.control_stream.recvmsg() {
                Ok(Message::ServerTestUpdate(update)) => update,
                Ok(other) => {return Err(eyre!("Expected ServerTestUpdate message iso {:?}", other));}
                Err(e) => {return Err(eyre!("Failed to communicate with server: {:?}", e));}
            };

            if let Some(update_cb) = &mut self.update_cb {
                update_cb(&update);
            }
            if update.total.testdone {
                return Ok(update);
            }
        }
    }

    pub fn run(&mut self) -> Result<ClientResult> {
        // notify server to start tests
        self.control_stream.sendmsg(&Message::ClientStartTest)
            .wrap_err("Failed to send ClientStartTest")?;
        
        // Unlock 'run' barrier to start all streams threads
        self.run_barrier.wait();

        match self.args.revert {
            true => self.run_download(),
            false => self.run_upload(),
        }
    }
}
