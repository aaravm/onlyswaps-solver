use std::collections::HashMap;
use std::pin::Pin;
use crate::config::{ConfigFile, NetworkConfig};
use crate::eth::ERC20Token::ERC20TokenInstance;
use crate::eth::Router::RouterInstance;
use crate::model::{ChainState, Transfer};
use crate::solver::ChainStateProvider;
use alloy::primitives::{Address, U256};
use alloy::providers::{DynProvider, Provider};
use futures::future::try_join_all;
use std::str::FromStr;
use std::sync::Arc;
use alloy::pubsub::SubscriptionStream;
use alloy::rpc::types::Header;
use eyre::eyre;
use futures::Stream;
use superalloy::provider::{MultiChainProvider, MultiProvider};
use tonic::async_trait;
use tonic::codegen::tokio_stream::adapters::Map;
use tonic::codegen::tokio_stream::StreamExt;

pub(crate) struct Chain<P> {
    chain_id: u64,
    account: Address,
    provider: Arc<P>,
    token: ERC20TokenInstance<Arc<P>>,
    router: RouterInstance<Arc<P>>,
}

impl Chain<&DynProvider> {
    pub async fn create_many<'a>(config: &ConfigFile, multi_provider: &'a MultiProvider<u64>) -> eyre::Result<HashMap<u64, Chain<&'a DynProvider>>> {
        let mut chains: HashMap<u64, Chain<&DynProvider>> = HashMap::new();

        for network_config in &config.networks {
            let chain_id = network_config.chain_id;
            let provider = multi_provider.get_provider(&chain_id).ok_or(eyre!("no provider configured for chain_id {}", chain_id))?;
            let chain = Chain::new(network_config.clone(), provider).await?;
            chain.withdraw_tokens().await?;
            chains.insert(chain_id, chain);
        }

        Ok(chains)
    }
}
impl<P> Chain<P>
where
    P: Provider,
{
  
    pub async fn new(network_config: NetworkConfig, provider: P) -> eyre::Result<Self> {
        let accounts = provider.get_accounts().await?;
        if accounts.len() == 0 {
            eyre::bail!("no accounts configured for private key!");
        }
        if accounts.len() > 1 {
            println!("warning: multiple accounts configured for private key chain; using the first");
        }
        let provider = Arc::new(provider);
        Ok(Chain {
            chain_id: network_config.chain_id,
            account: *accounts.first().expect("impossible because we just checked account"),
            token: ERC20TokenInstance::new(Address::from_str(&network_config.rusd_address)?, provider.clone()),
            router: RouterInstance::new(Address::from_str(&network_config.router_address)?, provider.clone()),
            provider: provider.clone(),
        })
    }

    pub async fn withdraw_tokens(&self) -> eyre::Result<()> {
        println!("checking funds for {}", self.chain_id);

        let min_balance = U256::from(1_000_000_000);
        let rusd_balance = self.token.balanceOf(self.account).call().await?;
        if rusd_balance > min_balance {
            println!("balance {} - not withdrawing tokens for chain_id {}", rusd_balance, &self.chain_id);
            return Ok(());
        }

        let tx = self.token.mint(self.account, min_balance).send().await?;
        tx.get_receipt().await?;
        println!("withdrew tokens for chain_id {}", &self.chain_id);
        Ok(())
    }
    
    pub async fn stream_block_numbers(&self) -> eyre::Result<Pin<Box<dyn Stream<Item = BlockEvent> + Send>>> {
        let chain_id = self.chain_id.clone();
        let stream = self.provider.subscribe_blocks()
            .await?
            .into_stream()
            .map(move |header| BlockEvent { chain_id, block_number: header.number });
        
        Ok(Box::pin(stream))
    }
}
pub(crate) struct BlockEvent { 
    pub chain_id: u64, 
    pub block_number: u64 
}

#[async_trait]
impl ChainStateProvider for Chain<&DynProvider> {
    async fn fetch_state(&self) -> eyre::Result<ChainState> {
        let native_balance = self.provider.get_balance(self.account).await?;
        let token_balance = self.token.balanceOf(self.account).call().await?;
        let unfulfilled = self.router.getUnfulfilledRequestIds().call().await?;
        let reqs = unfulfilled.into_iter().map(async |id| -> eyre::Result<Transfer> {
            let params = self.router.getTransferParameters(id).call().await?;
            Ok(Transfer { request_id: *id, params })
        });
        let transfers = try_join_all(reqs).await?;

        Ok(ChainState {
            native_balance,
            token_balance,
            transfers,
        })
    }
}
