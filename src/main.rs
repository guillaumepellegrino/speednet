#[macro_use]
extern crate lazy_static;

use eyre::{Result, WrapErr};
use clap::Parser;
use args::{Args, ArgsClient, ArgsServer, Subcommand};
mod args;
mod client;
mod message;
mod server;
mod pktgenerator;

async fn speednet_client(args: ArgsClient) -> Result<()> {
    let mut client = client::Client::new(args)?;
    client.run()
        .wrap_err("Failed to run speednet client")?;
    Ok(())
}

async fn speednet_server(args: ArgsServer) -> Result<()> {
    server::run(args)
        .wrap_err("Failed to run speednet server")?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.subcommand {
        Subcommand::Client(client) => speednet_client(client).await,
        Subcommand::Server(server) => speednet_server(server).await,
    }
}
