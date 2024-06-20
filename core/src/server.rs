use eyre::{eyre, Result, WrapErr};
use std::net::{TcpListener, TcpStream, SocketAddr, IpAddr, Ipv6Addr};
use std::sync::{
    Arc,
    RwLock,
};
use std::collections::HashMap;
use crate::{
    api::{Message, MessageIO},
    args::{ArgsClient, ArgsServer},
    pktgenerator,
    result::StreamResult,
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
    speedtests: HashMap<u32, Client>,
}

struct Client {
    config: ArgsClient,
}

impl Client {
    fn new(config: ArgsClient) -> Self {
        Self {
            config,
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
            Message::ClientStreamHello(testid, _streamid) => self.server_handle_client_start_stream(stream, testid),
            _ => Err(eyre!("Received an unexpected message: {:?}", msg)),
        }
    }

    fn server_handle_client_start_stream(&self, stream: TcpStream, testid: u32) -> Result<()> {
        println!("Test id: {}", testid);

        let mut server = self.shared.write().unwrap();
        let speedtest = match server.speedtests.get_mut(&testid) {
            Some(speedtest) => speedtest,
            None => {
                return Err(eyre!("Unknown testid {}", testid));
            },
        };

        let config = speedtest.config.clone();
        drop(server);

        let stream_result = std::sync::Mutex::new(StreamResult::default());
        if config.revert {
            pktgenerator::tcp_send(&config, stream, &stream_result);
        }
        else {
            pktgenerator::tcp_recv(&config, stream, &stream_result);
        }

        Ok(())
    }

    fn server_handle_client_hello(&self, mut stream: TcpStream, config: ArgsClient) -> Result<()> {
        println!("Client config: {:?}", config);

        // Create a new speedtest instance
        let mut server = self.shared.write().unwrap();
        let testid = server.next_testid;
        let speedtest = Client::new(config);
        server.speedtests.insert(testid, speedtest);
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

        Ok(())
    }

}

