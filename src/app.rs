use crate::executor::TradeExecutor;
use crate::model::BlockEvent;
use crate::network::Network;
use crate::solver::Solver;
use alloy::providers::DynProvider;
use futures::StreamExt;
use futures::future::try_join_all;
use futures::stream::select_all;
use std::collections::HashMap;

pub struct App {}
impl App {
    pub async fn start(networks: HashMap<u64, Network<DynProvider>>) -> eyre::Result<()> {
        let block_numbers = networks.values().map(|network| network.stream_block_numbers());
        let streams = try_join_all(block_numbers).await?;
        let mut stream = Box::pin(select_all(streams));

        let mut solver = Solver::from(&networks).await?;
        let executor = TradeExecutor::new(&networks);

        while let Some(BlockEvent { chain_id, .. }) = stream.next().await {
            let trades = solver.on_block(chain_id).await?;
            if !trades.is_empty() {
                println!("executing {} trades from chain {}", trades.len(), chain_id);
                executor.execute(trades).await;
            }
        }

        eyre::bail!("stream of blocks ended unexpectedly");
    }
}
