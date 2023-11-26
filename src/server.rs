use eyre::{eyre, Result, WrapErr};
use std::net::{TcpListener, TcpStream, SocketAddr, IpAddr, Ipv6Addr};
use std::sync::{
    RwLock,
};
use std::collections::HashMap;
use crate::{
    message::{Message, MessageIO},
    args::{ArgsClient, ArgsServer},
    pktgenerator,
};

#[derive(Default)]
struct Server {
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

lazy_static! {
    static ref SERVER: RwLock<Server> = RwLock::new(Server::default());
}

fn server_handle_client_hello(mut stream: TcpStream, config: ArgsClient) -> Result<()> {
    println!("Client config: {:?}", config);

    let mut server = SERVER.write().unwrap();
    let testid = server.next_testid;
    let speedtest = Speedtest::new(config);
    server.speedtests.insert(testid, speedtest);
    server.next_testid = testid + 1;
    drop(server);

    stream.sendmsg(&Message::ServerHello(testid))
        .wrap_err("Failed to send server hello")?;

    println!("Server hello sent");

    Ok(())
}

pub fn server_handle_tcp_download(stream: TcpStream, config: ArgsClient) -> Result<()> {
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

fn server_handle_tcp_upload(stream: TcpStream, config: ArgsClient) -> Result<()> {
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

fn server_handle_client_start_stream(stream: TcpStream, testid: u32) -> Result<()> {
    println!("Test id: {}", testid);

    let mut server = SERVER.write().unwrap();
    let speedtest = match server.speedtests.get_mut(&testid) {
        Some(speedtest) => speedtest,
        None => {
            return Err(eyre!("Unknown testid {}", testid));
        },
    };

    let config = speedtest.config.clone();
    drop(server);

    match config.revert {
        true => server_handle_tcp_download(stream, config)?,
        false => server_handle_tcp_upload(stream, config)?,
    };

    Ok(())
}

fn server_handle_new_client(mut stream: TcpStream) -> Result<()> {
    let msg = stream.recvmsg()
        .wrap_err("Failed to read client hello message")?;

    match msg {
        Message::ClientHello(config) => server_handle_client_hello(stream, config),
        Message::ClientStartStream(testid) => server_handle_client_start_stream(stream, testid),
        _ => Err(eyre!("Received an unexpected message: {:?}", msg)),
    }
}

pub fn run(args: ArgsServer) -> Result<()> {
    let ip_addr = match &args.hostname {
        Some(hostname) => hostname.parse::<IpAddr>().wrap_err("Invalid hostname")?,
        None => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    };
    let listen_addr = SocketAddr::new(ip_addr, args.port);

    //let (tx, rx) = std::sync::mpsc::channel();

    println!("speednet server listening on {:?}", listen_addr);
    let listener = TcpListener::bind(listen_addr)?;
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(e) => {
                println!("Connection error: {:?}", e);
                continue;
            }
        };
        std::thread::spawn(move || {
            if let Err(e) = server_handle_new_client(stream) {
                println!("Client error: {:?}", e);
            }
        });
    }

    Ok(())
}

