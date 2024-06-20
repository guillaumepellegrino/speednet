use eyre::{Result, WrapErr};
use clap::Parser;

use speednet_core::{
    Args,
    ArgsClient,
    ArgsServer,
    Client,
    Server,
    Subcommand,
};

fn speednet_client(args: ArgsClient) -> Result<()> {
    let mut client = Client::new(args)?;
    client.on_update(|update| {
        update.pretty_print();
    });
    let result = client.run()
        .wrap_err("Failed to run speednet client")?;
    print!("[Total]");
    result.pretty_print();
    
    Ok(())
}

fn speednet_server(args: ArgsServer) -> Result<()> {
    let mut server = Server::new(args)?;
    server.run()
        .wrap_err("Failed to run speednet server")?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.subcommand {
        Subcommand::Client(client) => speednet_client(client),
        Subcommand::Server(server) => speednet_server(server),
    }
}
