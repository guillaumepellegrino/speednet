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
    #[serde(default)]
    pub port: u16,

    /// Use UDP instead of TCP
    #[arg(short, long)]
    #[serde(default)]
    pub udp: bool,

    /// Download instead of Upload
    #[arg(short='R', long)]
    #[serde(default)]
    pub revert: bool,

    /// Set DSCP in packet IP Header
    #[arg(short, long)]
    #[serde(default)]
    pub dscp: Option<i32>,

    /// Set packet MARK
    #[arg(short, long)]
    #[serde(default)]
    pub mark: Option<i32>,

    /// Bind the specified IP Address
    #[arg(short='B', long)]
    #[serde(default)]
    pub bind: Option<String>,

    /// Set a target bandwidth
    #[arg(short, long, default_value_t=0)]
    #[serde(default)]
    pub bandwidth: u64,

    /// Set the number of open connections in parallel
    #[arg(short='P', long, default_value_t=1)]
    #[serde(default)]
    pub parallel: u32,

    /// Set the buffer len to use to send/recv packets
    #[arg(short, long, default_value_t=0)]
    #[serde(default)]
    pub len: u64,

    /// The test duration time
    #[arg(short, long, default_value_t=10)]
    #[serde(default)]
    pub time: u64,

    /// Draw speednet results in dataviewer
    #[arg(short, long)]
    #[serde(default)]
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

#[derive(Parser, Debug, Clone, PartialEq, Default)]
pub struct ArgsOrchestrate {
    /// List of scenarios (name or file path) to orchestrate
    pub scenarios: Vec<String>,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommand {
    /// Run in client mode, connecting to the specified server
    Client(ArgsClient),

    /// Run in server mode
    Server(ArgsServer),

    /// Orchestrate one or more tests scenarios
    Orchestrate(ArgsOrchestrate),
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub subcommand: Subcommand,
}


impl ArgsClient {
    /** Create new struct Arguments for speednet client */
    pub fn new(hostname: &str) -> Self {
        let args = [String::from("speednet"), String::from(hostname)];
        ArgsClient::parse_from(args)
    }

    /** Return the test bandwidth */
    pub fn get_bandwidth(&self) -> u64 {
        self.bandwidth
    }

    /** Return the socket buffer len */
    pub fn get_bufferlen(&self) -> u64 {
        self.len
    }

    /** Return the number of total packets to send for this test */
    pub fn get_totalpackets(&self) -> u64 {
        let bufferlen = self.get_bufferlen();
        self.time * self.bandwidth / (8 * bufferlen)
    }

    /** Return the number of total bytes to send for this test */
    pub fn get_total_bytes_expected(&self) -> u64 {
        self.time * self.bandwidth / 8
    }

    pub fn prepare_config(&mut self) {
        if self.bandwidth == 0 {
            self.bandwidth = match self.udp {
                true => 1000000,
                false => 10000000000,
            };
        }
        self.bandwidth /= self.parallel as u64;
        if self.len == 0 {
            self.len = match self.udp {
                true => 1472,
                false => std::cmp::min(1000000, self.bandwidth/80),
            };        
        }
    }

    pub fn print_config(&self) {
        let protocol = if self.udp {"UDP"} else {"TCP"};
        let direction = if self.revert {"download"} else {"upload"};
        eprintln!("Starting {} client using {}x {} streams at {}kbps with buffer len of {} bytes",
            protocol, self.parallel, direction, self.bandwidth/1000, self.len);
    }
}

impl ArgsServer {
    /** Create new struct Arguments for speednet server */
    pub fn new() -> Self {
        let args = [String::from("speednet")];
        ArgsServer::parse_from(args)
    }
}