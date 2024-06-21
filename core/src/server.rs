use eyre::{eyre, Result, WrapErr};
use std::net::{TcpListener, TcpStream, SocketAddr, IpAddr, Ipv6Addr};
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
        println!("speednet server listening on {:?}", local_addr);

        Ok(Self {
            listener,
            local_addr,
            inner: ServerInner::new()?
        })
    }

    /// Run the Speednet server forever
    ///
    /// The speednet server is waiting for new speednet client tcp control connections.
    /// Multiple speednet clients can connect at the same time.
    pub fn run(&mut self) -> Result<()> {
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
                if let Err(e) = me.server_handle_new_client(stream) {
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

    fn server_handle_new_client(&self, mut stream: TcpStream) -> Result<()> {
        let msg = stream.recvmsg()
            .wrap_err("Failed to read client hello message")?;

        match msg {
            Message::ClientHello(config) => self.server_handle_client_hello(stream, config),
            Message::ClientStreamHello(testid, streamid) => self.server_handle_client_start_stream(stream, testid, streamid),
            _ => Err(eyre!("Received an unexpected message: {:?}", msg)),
        }
    }

    fn server_handle_client_start_stream(&self, stream: TcpStream, testid: u32, streamid: u32) -> Result<()> {
        println!("Test id: {}", testid);

        let mut server = self.shared.write().unwrap();
        let client = server.clients.get_mut(&testid)
            .ok_or_else(|| eyre!("Unknown testid {}", testid))?;
        let config = client.config.clone();
        let stream_results = client.stream_results.clone();
        let testdone_barrier = client.testdone_barrier.clone();
        drop(server);

        let stream_result = stream_results.get(streamid as usize).expect("Unknown streamid");
        if config.revert {
            pktgenerator::tcp_send(&config, stream, &stream_result);
        }
        else {
            pktgenerator::tcp_recv(&config, stream, &stream_result);
        }
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

        // Collect stream results every second until time is elapsed
        let total_packets = config.get_totalpackets();
        let duration = Duration::from_secs(config.time);
        let now = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let elapsed = now.elapsed();
            if elapsed.as_secs() >= config.time {
                break;
            }
            let pktcount_expected = ((total_packets as u128 * elapsed.as_nanos()) / duration.as_nanos()) as u64;
            let client_result = ClientResult::collect_and_override(&stream_results, elapsed, pktcount_expected);
            stream.sendmsg(&Message::ServerTestUpdate(client_result))
                .wrap_err("Failed to send server test update")?;
        }

        // Send final result
        testdone_barrier.wait();
        let client_result = ClientResult::collect(&stream_results);
        stream.sendmsg(&Message::ServerTestUpdate(client_result))
            .wrap_err("Failed to send server test update")?;

        Ok(())
    }
}

