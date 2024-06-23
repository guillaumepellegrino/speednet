use serde::Deserialize;
use serde::Serialize;
use eyre::{eyre, Result, WrapErr};
use std::collections::HashMap;
use crate::ArgsClient;
use crate::Client;

pub struct Scenario {
    name: String,
    args: HashMap<String, ArgsClient>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct ScenarioEnv {
    #[serde(default)]
    var: HashMap<String, Environment>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct ScenarioCfg {
    #[serde(default)]
    var: HashMap<String, Environment>,
    client: HashMap<String, ClientCfg>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Environment {
    pub description: String,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct ClientCfg {
    pub hostname: String,
    pub port: Option<u16>,
    pub udp: Option<bool>,
    pub revert: Option<bool>,
    pub download: Option<bool>,
    pub upload: Option<bool>,
    pub dscp: Option<i32>,
    pub mark: Option<i32>,
    pub bind: Option<String>,
    pub bandwidth: Option<String>,
    pub parallel: Option<u32>,
    pub len: Option<u64>,
    pub expect: Option<Expect>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Expect {

}
impl ScenarioEnv {
    fn resolve_environment(&self, string: &mut String) -> Result<()> {
        let mut error = false;
        for (envar, info) in &self.var {
            let value = match std::env::var(envar) {
                Ok(value) => Some(value),
                Err(_) => info.default.clone(),
            };
            let value = match value {
                Some(value) => value,
                None => {
                    eprintln!("- '{}' is undefined ({})", envar, info.description);
                    error = true;
                    continue;
                }
            };
            eprintln!("- '{}' is {}  ({})", envar, value, info.description);
            let needle = format!("$({})", envar);
            *string = string.replace(&needle, &value);
        }
        if error {
            return Err(eyre!("Please define all environment variables"));
        }
        Ok(())
    }
}

impl ClientCfg {
    fn to_args(&self) -> ArgsClient {
        let mut args = ArgsClient::new(&self.hostname);
        if let Some(port) = self.port {
            args.port = port;
        }
        if let Some(udp) = self.udp {
            args.udp = udp;
        }
        if let Some(revert) = self.revert {
            args.revert = revert;
        }
        if let Some(download) = self.download {
            if download {
                args.revert = true;
            }
        }
        if let Some(upload) = self.upload {
            if upload {
                args.revert = false;
            }
        }
        args.dscp = self.dscp;
        args.mark = self.mark;
        args.bind = self.bind.clone();
        if let Some(bandwidth) = &self.bandwidth {
            args.bandwidth = crate::args::parse_number(bandwidth)
                .expect("bandwidth is not a number");
        }
        if let Some(parallel) = self.parallel {
            args.parallel = parallel;
        }
        if let Some(len) = self.len {
            args.len = len;
        }

        args
    }
}

impl Scenario {
    pub fn open(name: &str) -> Result<Self> {
        Self::open_file(name)
    }

    pub fn open_file(path: &str) -> Result<Self> {
        let mut string = std::fs::read_to_string(path)?;

        let env: ScenarioEnv = toml::from_str(&string)?;
        env.resolve_environment(&mut string)?;
        
        let cfg: ScenarioCfg = toml::from_str(&string)?;

        let mut args = HashMap::new();
        for (client_name, client) in &cfg.client {
            args.insert(client_name.clone(), client.to_args());
        }

        Ok(Self {
            name: String::from(path),
            args,
        })
    }

    pub fn run(&mut self, time: u64) -> Result<()> {
        eprintln!("[{}] Start scenario", self.name);

        let mut clients = vec!();
        for (client_name, args) in &mut self.args {
            eprintln!("[{}][{}] Init client", self.name, client_name);
            args.time = time;
            let mut client = Client::new(args.clone())
                .wrap_err_with(|| format!("[{}][{}] Failed to init client", self.name, client_name))?;
            client.set_name(&format!("[{}][{}]", self.name, client_name));

            clients.push(client);
        }

        let mut threads = vec!();
        while let Some(mut client) = clients.pop() {
            eprintln!("{} Run client", client.name());
            let thread = std::thread::spawn(move || {
                let client_name = client.name().to_string();
                client.on_update(move |update| {
                    eprint!("{}", client_name);
                    update.print_summary();
                });
                client.run()
            });
            threads.push(thread);
        }

        while let Some(thread) = threads.pop() {
            let result = thread.join()
                .expect("Failed to join client thread")
                .wrap_err_with(|| format!("[{}] Client returned an error", self.name))?;
            result.print_summary();
        }

        // TODO: check expected results, here

        eprintln!("[{}] Scenario done", self.name);

        Ok(())
    }


}
