use std::str::FromStr;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::signers::local::PrivateKeySigner;
use eyre::eyre;
use superalloy::provider::MultiProvider;
use crate::NetworkConfig;

pub(crate) async fn create_multiprovider(private_key: &str, networks: &Vec<NetworkConfig>) -> eyre::Result<MultiProvider<u64>>{
    if networks.is_empty() {
        return Err(eyre!("no networks configured"));
    }
    
    let signer = PrivateKeySigner::from_str(private_key)?;
    let wallet = EthereumWallet::new(signer);
    let mut multi_provider = MultiProvider::empty();

    for network in networks.iter() {
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
    
    println!("{} chain(s) have been configured", networks.len());

    Ok(multi_provider)
}
