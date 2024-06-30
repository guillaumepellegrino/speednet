use eyre::{eyre, Result, WrapErr};
use std::net::{TcpListener, TcpStream, UdpSocket, SocketAddr, IpAddr, Ipv6Addr};
use std::sync::{
    Arc,
    Barrier,
    RwLock,
    Mutex,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::{
    api::{Message, MessageIO},
    args::{ArgsClient, ArgsServer},
    pktgenerator,
    result::{ClientResult, StreamResult},
};

pub struct Server {
    listener: TcpListener,
    udp: Option<UdpSocket>,
    local_addr: SocketAddr,
    inner: ServerInner,
}

#[derive(Clone)]
pub struct ServerInner {
    shared: Arc<RwLock<SharedCtx>>,
}

#[derive(Default)]
struct SharedCtx {
    next_testid: u32,
    clients: HashMap<u32, Client>,
}

struct Client {
    testdone_barrier: Arc<Barrier>,
    config: ArgsClient,
    stream_results: Arc<Vec<Mutex<StreamResult>>>,
}

impl Client {
    fn new(config: ArgsClient) -> Self {
        let mut stream_results = vec!();
        for _streamid in 0 .. config.parallel {
            let stream_result = std::sync::Mutex::new(StreamResult::default());
            stream_results.push(stream_result);
        }

        Self {
            testdone_barrier: Arc::new(Barrier::new(config.parallel as usize + 1)),
            config,
            stream_results: Arc::new(stream_results),
        }
    }
}

impl Server {
    /// Intialize a new Speednet server according the provided configuration
    pub fn new(args: ArgsServer) -> Result<Self> {
        let ip_addr = match &args.bind {
            Some(hostname) => hostname.parse::<IpAddr>().wrap_err("Invalid hostname")?,
            None => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        };
        let listen_addr = SocketAddr::new(ip_addr, args.port);
        let listener = TcpListener::bind(listen_addr)?;
        let local_addr = listener.local_addr()?;

        // create UDP socket a BPF filter to receive only new connections
        let udp = UdpSocket::bind(listen_addr)?;
        crate::bpf::socket_attach_filter(&udp);
        println!("speednet server listening on {:?}", local_addr);

        Ok(Self {
            listener,
            udp: Some(udp),
            local_addr,
            inner: ServerInner::new()?
        })
    }

    /// Run the Speednet server forever
    ///
    /// The speednet server is waiting for new speednet client tcp control connections.
    /// Multiple speednet clients can connect at the same time.
    pub fn run(&mut self) -> Result<()> {
        let me = self.inner.clone();
        let udp = self.udp.take().unwrap();
        std::thread::spawn(move || {
            if let Err(e) = me.server_handle_new_udp_client(udp) {
                println!("Client error: {:?}", e);
            }
        });

        for stream in self.listener.incoming() {
            let me = self.inner.clone();
            let stream = match stream {
                Ok(stream) => stream,
                Err(e) => {
                    println!("Connection error: {:?}", e);
                    continue;
                }
            };
            std::thread::spawn(move || {
                if let Err(e) = me.server_handle_new_tcp_client(stream) {
                    println!("Client error: {:?}", e);
                }
            });
        }

        Ok(())

    }

    /// Return the local tcp socket address on which the
    /// server is listening.
    ///
    /// Useful for automated tests when starting server
    /// on a random port (= port 0).
    pub fn get_local_addr(&self) -> &SocketAddr {
        &self.local_addr
    }
}

impl ServerInner {
    fn new() -> Result<Self> {
        Ok(Self {
            shared: Arc::default(),
        })
    }

    fn server_handle_new_udp_client(&self, udp: UdpSocket) -> Result<()> {
        loop {
            let mut msg = [0; 32];
            udp.recv(&mut msg)
                .wrap_err("Failed to read message")?;
            println!("msg={:?}", msg);
        }

        //Ok(())
    }

    fn server_handle_new_tcp_client(&self, mut stream: TcpStream) -> Result<()> {
        stream.set_read_timeout(Some(Duration::from_millis(5000)))
            .wrap_err("Failed to set socket read timeout")?;
        stream.set_write_timeout(Some(Duration::from_millis(5000)))
            .wrap_err("Failed to set socket write timeout")?;

        let msg = stream.recvmsg()
            .wrap_err("Failed to read client hello message")?;

        match msg {
            Message::ClientHello(config) => self.server_handle_client_hello(stream, config),
            Message::ClientStreamHello(testid, streamid) => self.server_handle_client_start_stream(stream, testid, streamid),
            _ => Err(eyre!("Received an unexpected message: {:?}", msg)),
        }
    }

    fn server_handle_client_start_stream(&self, stream: TcpStream, testid: u32, streamid: u32) -> Result<()> {
        println!("Starting Test id: {}, Stream id: {}", testid, streamid);

        let mut server = self.shared.write().unwrap();
        let client = server.clients.get_mut(&testid)
            .ok_or_else(|| eyre!("Unknown testid {}", testid))?;
        let config = client.config.clone();
        let stream_results = client.stream_results.clone();
        let testdone_barrier = client.testdone_barrier.clone();
        drop(server);

        let stream_result = stream_results.get(streamid as usize).expect("Unknown streamid");
        if config.revert {
            pktgenerator::tcp_send(&config, stream);
        }
        else {
            pktgenerator::tcp_recv(&config, stream, &stream_result);
        }
        println!("DONE: Test id: {}, Stream id: {}", testid, streamid);
        testdone_barrier.wait();
        Ok(())
    }

    fn server_handle_client_hello(&self, mut stream: TcpStream, config: ArgsClient) -> Result<()> {
        println!("Client config: {:?}", config);

        // Create a new client instance
        let mut server = self.shared.write().unwrap();
        let testid = server.next_testid;
        let client = Client::new(config);
        let config = client.config.clone();
        let testdone_barrier = client.testdone_barrier.clone();
        let stream_results = client.stream_results.clone();
        server.clients.insert(testid, client);
        server.next_testid = testid + 1;
        drop(server);

        println!("Client {} accepted", testid);

        // Reply with Server Hello
        stream.sendmsg(&Message::ServerHello(testid))
            .wrap_err("Failed to send server hello")?;
        println!("Server hello sent");


        // Wait for ClientStartTest message
        let msg = stream.recvmsg()
            .wrap_err("Failed to receive 'ClientStartTest' message")?;
        if msg != Message::ClientStartTest {
            return Err(eyre!("Receive unexpected message: {:?}", msg));
        }

        if config.revert {
            // no needs to collect stats in case of download
            testdone_barrier.wait();
            println!("Closing Client {} ", testid);
            return Ok(());
        }

        // Collect stream results every second until time is elapsed
        let total_bytes_expected = config.get_total_bytes_expected();
        let duration = Duration::from_secs(config.time);
        let now = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let elapsed = now.elapsed();
            if elapsed.as_secs() >= config.time {
                break;
            }
            let bytes_expected = ((total_bytes_expected as u128 * (elapsed.as_nanos())) / duration.as_nanos()) as u64;
            let client_result = ClientResult::collect_and_override(&stream_results, elapsed, bytes_expected);
            stream.sendmsg(&Message::ServerTestUpdate(client_result))
                .wrap_err("Failed to send server test update")?;
        }

        // Send final result
        testdone_barrier.wait();
        let client_result = ClientResult::collect(&stream_results);

        stream.sendmsg(&Message::ServerTestUpdate(client_result))
            .wrap_err("Failed to send server test update")?;

        println!("Closing Client {}: Collect done", testid);
        Ok(())
    }
}

