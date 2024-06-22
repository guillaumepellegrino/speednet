use serde::Deserialize;
use serde::Serialize;
use eyre::{eyre, Result};
use std::collections::HashMap;
use crate::ArgsClient;


#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Scenario {
    #[serde(default)]
    environment: HashMap<String, Environment>,
    client: HashMap<String, ArgsClient>,
    #[serde(default)]
    expect: HashMap<String, Expect>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct ScenarioEnv {
    #[serde(default)]
    environment: HashMap<String, Environment>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Environment {
    pub description: String,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Expect {

}
impl ScenarioEnv {
    pub fn resolve_environment(&self, string: &mut String) -> Result<()> {
        let mut error = false;
        for (envar, environment) in &self.environment {
            let value = match std::env::var(envar) {
                Ok(value) => Some(value),
                Err(_) => environment.default.clone(),
            };
            let value = match value {
                Some(value) => value,
                None => {
                    eprintln!("- '{}' is undefined ({})", envar, environment.description);
                    error = true;
                    continue;
                }
            };
            eprintln!("- '{}' is {}  ({})", envar, value, environment.description);
            let needle = format!("$({})", envar);
            *string = string.replace(&needle, &value);
        }
        if error {
            return Err(eyre!("Please define all environment variables"));
        }
        Ok(())
    }

    pub fn print_mandatory_environment(&self) {
        eprintln!("Mandatory variables are:");
        for (envar, environment) in &self.environment {
            if environment.default.is_none() {
                eprintln!(" - '{}': {}", envar, environment.description);
            }
        }
    }
}

impl Scenario {
    pub fn open(name: &str) -> Result<Self> {
        Self::open_file(name)
    }

    pub fn open_file(path: &str) -> Result<Self> {
        let mut string = std::fs::read_to_string(path)?;

        let scenario: ScenarioEnv = toml::from_str(&string)?;
        scenario.resolve_environment(&mut string)?;
        
        let scenario: Scenario = toml::from_str(&string)?;
        Ok(scenario)
    }

    pub fn run(&mut self) -> Result<()> {
        for (client_name, client) in &self.client {
            eprintln!("Start client {}", client_name);
        }
        Ok(())
    }
}
