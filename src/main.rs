mod provider;
mod api;
mod eth;
mod config;
mod chain;
mod app;
mod solver;
mod model;
mod executor;

use std::str::FromStr;
use crate::provider::create_multiprovider;
use clap::Parser;
use eyre::eyre;
use alloy::signers::local::PrivateKeySigner;
use dotenv::dotenv;
use superalloy::provider::{MultiChainProvider, MultiProvider};
use crate::api::ApiServer;
use crate::app::App;
use crate::chain::{Chain};
use crate::config::{load_config_file, CliArgs, ConfigFile};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv().ok();
    let cli = CliArgs::parse();
    let config: ConfigFile = load_config_file(&cli);

    let signer = PrivateKeySigner::from_str(&cli.private_key)?;
    let multi_provider: MultiProvider<u64> = create_multiprovider(signer, &config.networks).await?;
    let chains = Chain::create_many(&config, &multi_provider).await?;

    // start some healthcheck and signal handlers
    let api_server = ApiServer::new(cli.port);
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    // listen for alllll the things!
    tokio::select! {
        res = App::start(chains) => {
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
