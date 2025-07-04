mod db;
mod events;
mod faucet;
mod plugin;
mod provider;

use crate::events::{EventListener, create_omnievent_plugin};
use crate::plugin::PluginServer;
use crate::provider::create_multiprovider;
use alloy::sol;
use axum::Router;
use axum::routing::get;
use clap::Parser;
use eyre::eyre;
use serde::Deserialize;
use shellexpand::tilde;
use std::fs;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
struct CliArgs {
    #[arg(
        short = 'c',
        long = "config-path",
        env = "SOLVER_CONFIG_PATH",
        default_value = "~/.solver/config.json"
    )]
    config_path: String,

    #[arg(short = 's', long = "private-key", env = "SOLVER_PRIVATE_KEY")]
    private_key: String,

    #[arg(short = 'p', long = "port", env = "SOLVER_PORT", default_value = "8080")]
    port: u16,
}

#[derive(Deserialize, Debug, Clone)]
struct ConfigFile {
    networks: Vec<NetworkConfig>,
}

#[derive(Deserialize, Debug, Clone)]
struct NetworkConfig {
    chain_id: u64,
    rpc_url: String,
    order_book_address: String,
}

sol!(
    #[sol(rpc)]
    ERC20FaucetToken,
    "onlysubs-solidity/out/ERC20FaucetToken.sol/ERC20FaucetToken.json"
);

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cli = CliArgs::parse();
    let config: ConfigFile = load_config_file(&cli);

    let app = Router::new().route("/health", get(healthcheck_handler));
    let listener = TcpListener::bind(("0.0.0.0", cli.port)).await?;

    let provider = create_multiprovider(&cli.private_key, &config.networks).await?;
    let omnievent_plugin = create_omnievent_plugin(Arc::new(provider))?;

    let plugin_port: u16 = 8089;
    let mut event_listener = EventListener::new(plugin_port)?;
    let plugin_server = PluginServer::new(vec![omnievent_plugin], plugin_port);
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    println!("Listening on port {}", cli.port);
    tokio::select! {
        res = plugin_server.start() => {
            match res {
                Ok(_) => Err(eyre!("plugin server stopped unexpectedly")),
                Err(e) => Err(eyre!("plugin server stopped unexpectedly: {}", e))
            }
        },

        res = event_listener.stream(&config.networks) => {
            match res {
                Ok(_) => Err(eyre!("event listener stopped unexpectedly")),
                Err(e) => Err(eyre!("event listener stopped unexpectedly: {}", e))
            }
        }

        res = axum::serve(listener, app) => {
            match res {
                Ok(_) => Err(eyre!("http server stopped unexpectedly")),
                Err(e) => Err(eyre!("http server stopped unexpectedly: {}", e))
            }
        }
        
        _ = sigterm.recv() => {
            println!("received SIGTERM, shutting down...");
            Ok(())
        },

        _ = sigint.recv() => {
            println!("received SIGINT, shutting down...");
            Ok(())
        },

        _ = tokio::signal::ctrl_c() => {
            println!("received ctrl+c, shutting down...");
            Ok(())
        },
    }
}

fn load_config_file(cli: &CliArgs) -> ConfigFile {
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

async fn healthcheck_handler() -> &'static str {
    "ok"
}
