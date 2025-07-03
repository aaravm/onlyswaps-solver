mod db;
mod faucet;

use std::error::Error;
use crate::db::InMemoryDatabase;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use axum::Router;
use axum::routing::get;
use clap::Parser;
use eyre::eyre;
use omnievent::event_manager::EventManager;
use omnievent::grpc::OmniEventServiceImpl;
use omnievent::proto_types::RegisterNewEventRequest;
use omnievent::proto_types::omni_event_service_client::OmniEventServiceClient;
use omnievent::proto_types::omni_event_service_server::OmniEventServiceServer;
use serde::Deserialize;
use shellexpand::tilde;
use std::fs;
use std::str::FromStr;
use std::sync::Arc;
use alloy::primitives::Address;
use bytes::Bytes;
use superalloy::provider::MultiProvider;
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

    if config.networks.is_empty() {
        return Err(eyre!("no networks configured"));
    }

    let signer = PrivateKeySigner::from_str(&cli.private_key)?;
    let wallet = EthereumWallet::new(signer);
    let mut multi_provider = MultiProvider::empty();

    for network in config.networks.iter().cloned() {
        let url = network.rpc_url.clone();
        let chainid = network.chain_id.clone();
        let provider = ProviderBuilder::new()
            .with_gas_estimation()
            .wallet(wallet.clone())
            .connect_ws(WsConnect::new(url))
            .await?
            .erased();
        multi_provider.extend::<Ethereum>([(chainid, provider)]);
    }
    println!("{} chain(s) have been configured", config.networks.len());

    let db = InMemoryDatabase::default();
    let mut event_manager = EventManager::new(Arc::new(multi_provider), db);
    event_manager.start();
    let omnievent = Arc::new(OmniEventServiceImpl::new(Arc::new(event_manager)));

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let mut client = OmniEventServiceClient::connect("http://127.0.0.1:8089").await.unwrap();
        for network in config.networks.iter().cloned() {
            let chain_id = network.chain_id;
            let orderbook_address = Address::from_str(network.order_book_address.clone().as_ref()).unwrap();
            let registration = client
                .register_event(RegisterNewEventRequest {
                    chain_id,
                    address: Bytes::from(orderbook_address.to_vec()).into(),
                    event_name: "SwapRequested".into(),
                    fields: vec![omnievent::proto_types::EventField {
                        sol_type: "uint256".into(),
                        indexed: false,
                    }],
                    block_safety: Default::default(),
                })
                .await;
            if let Err(e) = registration {
                eprintln!("failed to register event for chain {}: {}", chain_id, e);
            }
        }
    });

    let blockchain_plugin_socket = "127.0.0.1:8089".parse()?;
    let blockchain_plugin =
        tonic::transport::Server::builder().add_service(OmniEventServiceServer::from_arc(Arc::clone(&omnievent)));

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

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

        res = blockchain_plugin.serve(blockchain_plugin_socket) => {
            match res {
                Ok(_) => Err(eyre!("blockchain event listener stopped unexpectedly")),
                Err(e) => Err(eyre!("blockchain event listener stopped unexpectedly: {}", e))
            }
        },

        res = axum::serve(listener, app) => {
            match res {
                Ok(_) => Err(eyre!("http server stopped unexpectedly")),
                Err(e) => Err(eyre!("http server stopped unexpectedly: {}", e))
            }
        }
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
