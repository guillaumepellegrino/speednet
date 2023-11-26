/// speednet command line arguments
///
use clap::Parser;
use serde::Deserialize;
use serde::Serialize;

#[derive(Parser, Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct ArgsClient {
    /// speednet server hostname
    pub hostname: String,

    /// speednet server control port
    #[arg(short, long, default_value_t=4000)]
    pub port: u16,

    /// Use UDP instead of TCP
    #[arg(short, long)]
    pub udp: bool,

    /// Download instead of Upload
    #[arg(short='R', long)]
    pub revert: bool,

    /// Set DSCP in packet IP Header
    #[arg(short, long)]
    pub dscp: Option<i32>,

    /// Set packet MARK
    #[arg(short, long)]
    pub mark: Option<i32>,

    /// Bind the specified IP Address
    #[arg(short='B', long)]
    pub bind: Option<String>,

    /// Set a target bandwidth
    #[arg(short, long)]
    bandwidth: Option<u64>,

    /// Set the number of open connections in parallel
    #[arg(short='P', long, default_value_t=1)]
    pub parallel: u32,

    /// Set the buffer len to use to send/recv packets
    #[arg(short, long, default_value_t=100000)]
    len: u64,

    /// The test duration time
    #[arg(short, long, default_value_t=10)]
    pub time: u64,

    /// Draw speednet results in dataviewer
    #[arg(short, long)]
    pub view: bool,
}

#[derive(Parser, Debug, Clone, PartialEq, Default)]
pub struct ArgsServer {
    /// Bind the specified IP Address
    pub bind: Option<String>,

    /// speednet server control port
    #[arg(short, long, default_value_t=4000)]
    pub port: u16,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommand {
    /// Run in client mode, connecting to the specified server
    Client(ArgsClient),

    /// Run in server mode
    Server(ArgsServer),
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub subcommand: Subcommand,
}


impl ArgsClient {
    /** Return the test bandwidth */
    pub fn get_bandwidth(&self) -> u64 {
        self.bandwidth.unwrap_or(0)
    }

    /** Return the socket buffer len */
    pub fn get_bufferlen(&self) -> u64 {
        let len = std::cmp::min(self.len, 10*1000*1000);
        std::cmp::max(len, 10)
    }

    /** Return the number of total packets to send for this test */
    pub fn get_totalpackets(&self) -> u64 {
        let bandwidth = match self.bandwidth {
            Some(bandwidth) => bandwidth,
            None => {return 0;},
        };
        let bufferlen = self.get_bufferlen();

        self.time * bandwidth / (8 * bufferlen)
    }
}
