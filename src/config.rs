use clap::Parser;
use serde::Deserialize;
use shellexpand::tilde;
use std::fs;
use alloy::primitives::U256;

#[derive(Parser, Debug)]
pub(crate) struct CliArgs {
    #[arg(short = 'c', long = "config", env = "SOLVER_CONFIG_PATH", default_value = "~/.solver/config.json")]
    pub config_path: String,

    #[arg(short = 's', long = "private-key", env = "SOLVER_PRIVATE_KEY")]
    pub private_key: String,

    #[arg(short = 'p', long = "port", env = "SOLVER_PORT", default_value = "8081")]
    pub port: u16,

    #[arg(short = 'i', long = "solver-id", env = "SOLVER_ID", default_value = "1")]
    pub solver_id: u8,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct ConfigFile {
    pub networks: Vec<NetworkConfig>,
    pub solver_config: Option<SolverConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct SolverConfig {
    pub threshold_multiplier: f64, // Multiplier for min_allowed_cost (e.g., 2.0 = 2x threshold)
    pub solver_name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct NetworkConfig {
    pub chain_id: u64,
    pub rpc_url: String,
    pub rusd_address: String,
    pub router_address: String,
}

pub(crate) fn load_config_file(cli: &CliArgs) -> ConfigFile {
    println!("loading config file {}", cli.config_path);
    match fs::read(tilde(&cli.config_path).into_owned()) {
        Ok(contents) => serde_json::from_slice(&contents).unwrap_or_else(|_| panic!("failed to parse config file at {}", cli.config_path)),
        Err(err) => panic!("failed to read config file at {}: {:?}", cli.config_path, err.to_string()),
    }
}
