use eyre::{eyre, Result, WrapErr};
use std::net::{TcpListener, TcpStream, SocketAddr, IpAddr, Ipv6Addr};
use std::sync::{
    Arc,
    RwLock,
};
use std::collections::HashMap;
use crate::{
    message::{Message, MessageIO},
    args::{ArgsClient, ArgsServer},
    pktgenerator,
};

#[derive(Default, Clone)]
pub struct Server {
    inner: Arc<RwLock<ServerInner>>,
    args: ArgsServer,
}

#[derive(Default)]
struct ServerInner {
    next_testid: u32,
    speedtests: HashMap<u32, Speedtest>,
}

struct Speedtest {
    config: ArgsClient,
}

impl Speedtest {
    fn new(config: ArgsClient) -> Self {
        Self {
            config,
        }
    }
}

impl Server {
    pub fn new(args: ArgsServer) -> Result<Self> {
        Ok(Self {
            inner: Arc::default(),
            args,
        })
    }

    pub fn run(&self) -> Result<()> {
        let ip_addr = match &self.args.bind {
            Some(hostname) => hostname.parse::<IpAddr>().wrap_err("Invalid hostname")?,
            None => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        };
        let listen_addr = SocketAddr::new(ip_addr, self.args.port);

        //let (tx, rx) = std::sync::mpsc::channel();

        println!("speednet server listening on {:?}", listen_addr);
        let listener = TcpListener::bind(listen_addr)?;
        for stream in listener.incoming() {
            let me = self.clone();
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

    fn server_handle_new_client(&self, mut stream: TcpStream) -> Result<()> {
        let msg = stream.recvmsg()
            .wrap_err("Failed to read client hello message")?;

        match msg {
            Message::ClientHello(config) => self.server_handle_client_hello(stream, config),
            Message::ClientStreamHello(testid, _streamid) => self.server_handle_client_start_stream(stream, testid),
            _ => Err(eyre!("Received an unexpected message: {:?}", msg)),
        }
    }

    pub fn server_handle_tcp_download(&self, stream: TcpStream, config: ArgsClient) -> Result<()> {
        println!("Handle TCP Download");
        let result = pktgenerator::tcp_send(&config, stream, |update| {
            println!("Elapsed: {}", update.elapsed.as_secs());
            println!("pktsent: {}", update.pktcount);
            println!("expected: {}", update.pktcount_expected);
            println!("");
        })?;
        println!("Handle TCP Download done");
        println!("Elapsed: {}", result.elapsed.as_secs());
        println!("Pkt Sent: {}", result.pktcount);
        Ok(())
    }

    fn server_handle_tcp_upload(&self, stream: TcpStream, config: ArgsClient) -> Result<()> {
        println!("Handle TCP Upload");
        let result = pktgenerator::tcp_recv(&config, stream, |update| {
            println!("Elapsed: {}", update.elapsed.as_secs());
            println!("pktrecv: {}", update.pktcount);
            println!("");
        })?;

        println!("Handle TCP Upload done");
        println!("Elapsed: {}", result.elapsed.as_secs());
        println!("Pkt Recv: {}", result.pktcount);
        Ok(())
    }

    fn server_handle_client_start_stream(&self, stream: TcpStream, testid: u32) -> Result<()> {
        println!("Test id: {}", testid);

        let mut server = self.inner.write().unwrap();
        let speedtest = match server.speedtests.get_mut(&testid) {
            Some(speedtest) => speedtest,
            None => {
                return Err(eyre!("Unknown testid {}", testid));
            },
        };

        let config = speedtest.config.clone();
        drop(server);

        match config.revert {
            true => self.server_handle_tcp_download(stream, config)?,
            false => self.server_handle_tcp_upload(stream, config)?,
        };

        Ok(())
    }

    fn server_handle_client_hello(&self, mut stream: TcpStream, config: ArgsClient) -> Result<()> {
        println!("Client config: {:?}", config);

        // Create a new speedtest instance
        let mut server = self.inner.write().unwrap();
        let testid = server.next_testid;
        let speedtest = Speedtest::new(config);
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

