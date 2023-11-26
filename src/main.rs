use eyre::{Result, WrapErr};
use clap::Parser;
use args::{Args, ArgsClient, ArgsServer, Subcommand};
mod args;
mod client;
mod message;
mod server;
mod pktgenerator;

fn speednet_client(args: ArgsClient) -> Result<()> {
    let mut client = client::Client::new(args)?;
    client.run()
        .wrap_err("Failed to run speednet client")?;
    Ok(())
}

fn speednet_server(args: ArgsServer) -> Result<()> {
    let server = server::Server::new(args)?;
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
