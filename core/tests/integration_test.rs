use speednet_core::{
    ArgsClient,
    ArgsServer,
    Client,
    Server,
    result::ClientResult,
};

enum IPVersion {
    V4,
    V6,
}

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

fn run1client<F>(ipversion: IPVersion, setup: F) -> ClientResult
where F:FnOnce(&mut ArgsClient)
{
    let port = server_run();

    let addr = match ipversion {
        IPVersion::V4 => "127.0.0.1",
        IPVersion::V6 => "::1",
    };
    let mut args = ArgsClient::new(addr);
    args.time = 2;
    args.port = port;
    setup(&mut args);

    let mut client = Client::new(args).unwrap();
    let result = client.run().unwrap();
    eprintln!("Result: {:?}", result);
    eprintln!("Througput: {}kbps", result.total.get_througtput()/1000);
    result
}


#[test]
fn test_tcp4_ul1_10kbps() {
    let result = run1client(IPVersion::V4, |args| {
        args.revert = false;
        args.bandwidth = 100000;
    });
    assert!(result.total().throughput_is(100000, 2));
}

#[test]
fn test_tcp4_ul1_100kbps() {
    let result = run1client(IPVersion::V4, |args| {
        args.revert = false;
        args.bandwidth = 1000000;
    });
    assert!(result.total().throughput_is(1000000, 2));
}

#[test]
fn test_tcp4_ul1_1mbps() {
    let result = run1client(IPVersion::V4, |args| {
        args.revert = false;
        args.bandwidth = 10000000;
    });
    assert!(result.total().throughput_is(10000000, 2));
}

#[test]
fn test_tcp4_ul4_1mbps() {
    let result = run1client(IPVersion::V4, |args| {
        args.revert = false;
        args.bandwidth = 10000000;
        args.parallel = 4;
    });
    assert!(result.total().throughput_is(10000000, 2));
}

/*
#[test]
fn test_tcp4_ul16_1mbps() {
    let result = run1client(IPVersion::V4, |args| {
        args.revert = false;
        args.bandwidth = 10000000;
        args.parallel = 16;
    });
    assert!(result.total().throughput_is(10000000, 2));
}
*/

#[test]
fn test_tcp6_ul1_1mbps() {
    let result = run1client(IPVersion::V6, |args| {
        args.revert = false;
        args.bandwidth = 10000000;
    });
    assert!(result.total().throughput_is(10000000, 2));
}

#[test]
fn test_tcp4_dl1_10kbps() {
    let result = run1client(IPVersion::V4, |args| {
        args.revert = true;
        args.bandwidth = 100000;
    });
    assert!(result.total().throughput_is(100000, 2));
}

#[test]
fn test_tcp4_dl1_100kbps() {
    let result = run1client(IPVersion::V4, |args| {
        args.revert = true;
        args.bandwidth = 1000000;
    });
    assert!(result.total().throughput_is(1000000, 2));
}

#[test]
fn test_tcp4_dl1_1mbps() {
    let result = run1client(IPVersion::V4, |args| {
        args.revert = true;
        args.bandwidth = 10000000;
    });
    assert!(result.total().throughput_is(10000000, 2));
}

#[test]
fn test_tcp4_dl4_1mbps() {
    let result = run1client(IPVersion::V4, |args| {
        args.revert = true;
        args.bandwidth = 10000000;
        args.parallel = 4;
    });
    assert!(result.total().throughput_is(10000000, 2));
}

#[test]
fn test_tcp4_dl16_1mbps() {
    let result = run1client(IPVersion::V4, |args| {
        args.revert = true;
        args.bandwidth = 10000000;
        args.parallel = 16;
    });
    assert!(result.total().throughput_is(10000000, 2));
}

#[test]
fn test_tcp6_dl1_1mbps() {
    let result = run1client(IPVersion::V6, |args| {
        args.revert = true;
        args.bandwidth = 10000000;
    });
    assert!(result.total().throughput_is(10000000, 2));
}
