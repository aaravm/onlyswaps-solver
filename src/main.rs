mod network;
mod api;
mod eth;
mod config;
mod app;
mod solver;
mod model;
mod executor;
mod util;

use clap::Parser;
use eyre::eyre;
use dotenv::dotenv;
use crate::api::ApiServer;
use crate::app::App;
use crate::config::{load_config_file, CliArgs, ConfigFile};
use crate::network::Network;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv().ok();
    let cli = CliArgs::parse();
    let config: ConfigFile = load_config_file(&cli);
    let networks = Network::create_many(&cli.private_key, &config.networks).await?;

    // start some healthcheck and signal handlers
    let api_server = ApiServer::new(cli.port);
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    // listen for alllll the things!
    tokio::select! {
        res = App::start(networks) => {
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
