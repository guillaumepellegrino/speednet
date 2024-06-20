use speednet_core::{
    ArgsClient,
    ArgsServer,
    Client,
    Server,
    result::ClientResult,
};

// Run a new server listening on a random port
fn server_run() -> u16 {
    let mut args = ArgsServer::new();
    args.port = 0;
    let mut server = Server::new(args).unwrap();
    let local = server.get_local_addr();
    let port = local.port();
    assert!(port != 0);

    std::thread::spawn(move || server.run().unwrap());
    port
}

fn run1client<F>(setup: F) -> ClientResult
where F:FnOnce(&mut ArgsClient)
{
    let port = server_run();

    let mut args = ArgsClient::new("127.0.0.1");
    args.time = 2;
    args.port = port;
    setup(&mut args);

    let mut client = Client::new(args).unwrap();
    let result = client.run().unwrap();
    eprintln!("result: {:?}", result);
    result
}

#[test]
fn test_tcp4_ul1() {
    let result = run1client(|args| {
        args.revert = false;
        args.bandwidth = 10000000;
    });
    assert!(result.total().throughput_is(10000000, 2));
}

#[test]
fn test_tcp4_dl1() {
    let result = run1client(|args| {
        args.revert = true;
        args.bandwidth = 10000000;
    });
    assert!(result.total().throughput_is(10000000, 2));
}
