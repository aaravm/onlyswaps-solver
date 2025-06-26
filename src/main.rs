use std::fs;
use clap::Parser;
use serde::Deserialize;
use shellexpand::tilde;

#[derive(Parser, Debug)]
struct CliArgs {
    #[arg(short = 'c', long = "config-path", env = "SOLVER_CONFIG_PATH", default_value = "~/.solver/config.json")]
    config_path: String    
}

#[derive(Deserialize, Debug)]
struct ConfigFile {
    networks: Vec<NetworkConfig>
}

#[derive(Deserialize, Debug)]
struct NetworkConfig {
    chain_id: String,
    name: String,
    rpc_url: String
}

fn main() {
    let cli = CliArgs::parse();
    
    let config: ConfigFile = match fs::read(tilde(&cli.config_path).into_owned()) {
        Ok(contents) => serde_json::from_slice(&contents).expect(format!("failed to parse config file at {}", cli.config_path).as_str()),
        Err(err) => panic!("failed to read config file at {}: {:?}", cli.config_path, err.to_string()),
    };
    
    println!("number of networks: {}", config.networks.len());
}