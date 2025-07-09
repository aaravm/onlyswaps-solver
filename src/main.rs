mod db;
mod events;
mod plugin;
mod provider;
mod api;
mod eth;
mod config;
mod chain;

use std::collections::HashMap;
use std::str::FromStr;
use crate::events::{PluginHandler, create_omnievent_plugin};
use crate::plugin::PluginServer;
use crate::provider::create_multiprovider;
use clap::Parser;
use eyre::eyre;
use std::sync::Arc;
use alloy::primitives::{Address, U256};
use alloy::signers::local::PrivateKeySigner;
use dotenv::dotenv;
use superalloy::provider::MultiProvider;
use crate::api::ApiServer;
use crate::chain::{Chain, ChainConfig};
use crate::config::{load_config_file, CliArgs, ConfigFile};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv().ok();
    let cli = CliArgs::parse();
    let config: ConfigFile = load_config_file(&cli);

    let signer = PrivateKeySigner::from_str(&cli.private_key)?;
    let our_address = signer.address();
    let provider: MultiProvider<u64> = create_multiprovider(signer, &config.networks).await?;
    let mut chains: HashMap<U256, Chain> = HashMap::new();

    for n in config.networks {
        let chain = Chain::new(&provider, &ChainConfig {
            chain_id: n.chain_id,
            our_address,
            router_addr: Address::from_str(&n.router_address)?,
            token_addr: Address::from_str(&n.rusd_address)?,
        })?;
        chain.withdraw_tokens().await?;
        chains.insert(U256::from(n.chain_id), chain);
    }

    // connect grpc for event listening plugins
    let plugin_port: u16 = 8089;
    let omnievent_plugin = create_omnievent_plugin(Arc::new(provider))?;
    let plugin_server = PluginServer::new(vec![omnievent_plugin], plugin_port);
    let mut plugin_handler = PluginHandler::new(plugin_port)?;

    // start some healthcheck and signal handlers
    let api_server = ApiServer::new(cli.port);
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    // listen for alllll the things!
    tokio::select! {
        res = plugin_server.start() => {
            match res {
                Ok(_) => Err(eyre!("plugin server stopped unexpectedly")),
                Err(e) => Err(eyre!("plugin server stopped unexpectedly: {}", e))
            }
        },

        res = plugin_handler.stream(&chains) => {
            match res {
                Ok(_) => Err(eyre!("event listener stopped unexpectedly")),
                Err(e) => Err(eyre!("event listener stopped unexpectedly: {}", e))
            }
        }

        res = api_server.start() => {
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
