use crate::config::NetworkConfig;
use crate::eth::ERC20Token::ERC20TokenInstance;
use crate::eth::Router::RouterInstance;
use crate::model::{BlockEvent, ChainState, Transfer};
use crate::solver::ChainStateProvider;
use alloy::network::EthereumWallet;
use alloy::primitives::{Address, U256};
use alloy::providers::{DynProvider, Provider, ProviderBuilder, WsConnect};
use alloy::signers::local::PrivateKeySigner;
use futures::Stream;
use futures::future::try_join_all;
use itertools::Itertools;
use std::collections::HashMap;
use std::pin::Pin;
use std::str::FromStr;
use async_trait::async_trait;
use futures::StreamExt;

pub(crate) struct Network<P> {
    pub chain_id: u64,
    pub provider: P,
    pub own_addr: Address,
    pub token: ERC20TokenInstance<P>,
    pub router: RouterInstance<P>,
}

impl Network<DynProvider> {
    pub async fn create_many(private_key: &str, network_configs: &Vec<NetworkConfig>) -> eyre::Result<HashMap<u64, Self>> {
        let mut networks = HashMap::new();
        let signer = PrivateKeySigner::from_str(private_key)?;

        for config in network_configs.iter() {
            let network = Network::new(&signer, config).await?;
            network.withdraw_tokens().await?;

            networks.insert(config.chain_id, network);
        }

        println!("{} chain(s) have been configured", network_configs.len());
        Ok(networks)
    }

    pub async fn new(signer: &PrivateKeySigner, config: &NetworkConfig) -> eyre::Result<Self> {
        let url = config.rpc_url.clone();
        let chain_id = config.chain_id.clone();
        let provider = ProviderBuilder::new()
            .with_gas_estimation()
            .wallet(EthereumWallet::new(signer.clone()))
            .connect_ws(WsConnect::new(url))
            .await?
            .erased();
        let own_addr = signer.address();

        println!("own addr: {}", own_addr);
        Ok(Self {
            token: ERC20TokenInstance::new(config.rusd_address.parse()?, provider.clone()),
            router: RouterInstance::new(config.router_address.parse()?, provider.clone()),
            chain_id,
            provider,
            own_addr,
        })
    }
}

impl<P: Provider> Network<P> {
    pub async fn withdraw_tokens(&self) -> eyre::Result<()> {
        println!("checking funds for {}", self.chain_id);

        let min_balance = U256::from_str("1_000_000_000_000_000_000_000_000_000")?;
        let rusd_balance = self.token.balanceOf(self.own_addr).call().await?;
        if rusd_balance > min_balance {
            println!("balance {} - not withdrawing tokens for chain_id {}", rusd_balance, &self.chain_id);
            return Ok(());
        }

        let tx = self.token.mint(self.own_addr, min_balance).send().await?;
        let hash = tx.watch().await?;
        println!("withdrew tokens for chain_id {}: {}", &self.chain_id, hash);
        Ok(())
    }

    pub async fn stream_block_numbers(&self) -> eyre::Result<Pin<Box<dyn Stream<Item = BlockEvent> + Send>>> {
        let chain_id = self.chain_id.clone();
        let stream = self.provider.subscribe_blocks().await?.into_stream().map(move |header| BlockEvent {
            chain_id,
            block_number: header.number,
        });

        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl ChainStateProvider for Network<DynProvider> {
    async fn fetch_state(&self) -> eyre::Result<ChainState> {
        let token_addr = self.token.address().clone();
        let native_balance = self.provider.get_balance(self.own_addr).await?;
        let token_balance = self.token.balanceOf(self.own_addr).call().await?;
        let already_fulfilled = self.router.getFulfilledTransfers().call().await?.into_iter().map_into().collect_vec();

        let unfulfilled = self.router.getUnfulfilledSolverRefunds().call().await?;
        let reqs = unfulfilled.into_iter().map(async |id| -> eyre::Result<Transfer> {
            let params = self.router.getTransferParameters(id).call().await?;
            Ok(Transfer { request_id: *id, params })
        });
        let transfers = try_join_all(reqs).await?;

        Ok(ChainState {
            token_addr,
            native_balance,
            token_balance,
            transfers,
            already_fulfilled,
        })
    }
}
