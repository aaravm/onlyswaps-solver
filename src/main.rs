use clap::Parser;
use serde::Deserialize;
use shellexpand::tilde;
use std::fs;
use std::io::Error;
use axum::Router;
use axum::routing::get;
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

    #[arg(
        short = 'p',
        long = "port",
        env = "ONLYSWAPS_VERIFIER_PORT",
        default_value = "8080"
    )]
    port: u16,
}

#[derive(Deserialize, Debug)]
struct ConfigFile {
    networks: Vec<NetworkConfig>,
}

#[derive(Deserialize, Debug)]
struct NetworkConfig {
    chain_id: String,
    name: String,
    rpc_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = CliArgs::parse();

    let config: ConfigFile = match fs::read(tilde(&cli.config_path).into_owned()) {
        Ok(contents) => serde_json::from_slice(&contents)
            .expect(format!("failed to parse config file at {}", cli.config_path).as_str()),
        Err(err) => panic!(
            "failed to read config file at {}: {:?}",
            cli.config_path,
            err.to_string()
        ),
    };
    
    let app = Router::new().route("/health", get(healthcheck_handler));
    let listener = TcpListener::bind(("0.0.0.0", cli.port)).await?;

    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    println!("{} chain(s) have been configured", config.networks.len());
    println!("Listening on port {}", cli.port);
    tokio::select! {
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

            err = axum::serve(listener, app) => {
                eprintln!("axum stopped unexpectedly...");
                err
            }
        }
}

async fn healthcheck_handler() -> &'static str {
    "ok"
}
