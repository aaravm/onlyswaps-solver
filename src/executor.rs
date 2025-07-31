use crate::eth::Router::RouterInstance;
use crate::model::Trade;
use crate::network::Network;
use crate::util::normalise_chain_id;
use alloy::primitives::TxHash;
use alloy::providers::Provider;
use std::collections::HashMap;
use crate::eth::ERC20FaucetToken::ERC20FaucetTokenInstance;

pub(crate) struct TradeExecutor<'a, P> {
    routers: HashMap<u64, &'a RouterInstance<P>>,
    tokens: HashMap<u64, &'a ERC20FaucetTokenInstance<P>>,
}

impl<'a, P: Provider> TradeExecutor<'a, P> {
    pub fn new(networks: &'a HashMap<u64, Network<P>>) -> Self {
        let routers = networks.iter().map(|(chain_id, net)| (*chain_id, &net.router)).collect();
        let tokens = networks.iter().map(|(chain_id, net)| (*chain_id, &net.token)).collect();
        Self { routers, tokens }
    }
    pub async fn execute(&self, trades: Vec<Trade>) {
        for trade in trades {
            // get the contract bindings for the destination chain
            let router = self
                .routers
                .get(&normalise_chain_id(trade.dest_chain_id))
                .expect("somehow didn't have a router binding for a solved trade");
            let token = self
                .tokens
                .get(&normalise_chain_id(trade.dest_chain_id))
                .expect("somehow didn't have a token binding for a solved trade");

            // approve the movement of funds from the ERC20, but don't wait for the tx receipt;
            // in theory, they should be processed in the same block in nonce order
            if let Err(e) = token.approve(*router.address(), trade.amount).send().await {
                println!("error approving trade: {}", e);
            }

            // actually send the funds via the router contract
            let relay: eyre::Result<TxHash> = async {
                let tx = router
                    .relayTokens(
                        trade.token_addr,
                        trade.recipient_addr,
                        trade.amount,
                        trade.request_id.into(),
                        trade.src_chain_id,
                    )
                    .send()
                    .await?;
                let receipt = tx.watch().await?;
                Ok(receipt)
            }
            .await;
            match relay {
                Ok(_) => println!("successfully traded {} on {}", trade.amount, trade.dest_chain_id),
                Err(e) => println!("error trading {} on {}: {}", trade.amount, trade.dest_chain_id, e),
            }
        }
    }
}
