use std::fs;
use clap::Parser;
use serde::Deserialize;
use shellexpand::tilde;

#[derive(Parser, Debug)]
pub(crate) struct CliArgs {
    #[arg(
        short = 'c',
        long = "config-path",
        env = "SOLVER_CONFIG_PATH",
        default_value = "~/.solver/config.json"
    )]
    pub config_path: String,

    #[arg(short = 's', long = "private-key", env = "SOLVER_PRIVATE_KEY")]
    pub private_key: String,

    #[arg(short = 'p', long = "port", env = "SOLVER_PORT", default_value = "8080")]
    pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct ConfigFile {
    pub networks: Vec<NetworkConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct NetworkConfig {
    pub chain_id: u64,
    pub rpc_url: String,
    pub rusd_address: String,
    pub bridge_address: String,
    pub router_address: String,
}

pub(crate) fn load_config_file(cli: &CliArgs) -> ConfigFile {
    match fs::read(tilde(&cli.config_path).into_owned()) {
        Ok(contents) => serde_json::from_slice(&contents)
            .expect(format!("failed to parse config file at {}", cli.config_path).as_str()),
        Err(err) => panic!(
            "failed to read config file at {}: {:?}",
            cli.config_path,
            err.to_string()
        ),
    }
}
