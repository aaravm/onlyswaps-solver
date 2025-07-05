use crate::config::NetworkConfig;
use crate::eth::ERC20Token;
use alloy::primitives::{Address, U256};
use alloy::signers::local::PrivateKeySigner;
use eyre::eyre;
use std::str::FromStr;
use superalloy::provider::{MultiChainProvider, MultiProvider};

pub(crate) async fn fund_wallets(networks: &Vec<NetworkConfig>, multi_provider: &MultiProvider<u64>, private_key: &str) -> eyre::Result<()> {
    let sk = PrivateKeySigner::from_str(private_key)?;
    let our_address = sk.address();
    for network in networks {
        withdraw_funds(multi_provider, our_address, &network).await?;
    }
    Ok(())
}

async fn withdraw_funds(multi_provider: &MultiProvider<u64>, our_address: Address, network: &NetworkConfig) -> eyre::Result<()> {
    println!("checking funds for {}", network.chain_id);

    let provider = multi_provider.get_ethereum_provider(&network.chain_id).ok_or(eyre!("No provider for network"))?;
    let rusd_address = network.rusd_address.parse()?;
    println!("rusd contract address is {}", &rusd_address);
    let contract = ERC20Token::new(rusd_address, provider);

    let rusd_balance = contract.balanceOf(our_address).call().await?;
    if rusd_balance > U256::from(0) {
        println!("balance {} - not withdrawing tokens for chain_id {}", rusd_balance, &network.chain_id);
        return Ok(());
    }

    println!("withdrawing some tokens to address {}", our_address);
    let tx = contract.mint(our_address, U256::from(1000)).send().await?;
    tx.get_receipt().await?;
    println!("withdrew tokens for chain_id {}", &network.chain_id);
    Ok(())
}
