use crate::chain::{BlockEvent, Chain};
use crate::executor::TradeExecutor;
use crate::solver::Solver;
use alloy::providers::DynProvider;
use futures::stream::select_all;
use std::collections::HashMap;
use futures::future::try_join_all;
use tonic::codegen::tokio_stream::StreamExt;

pub struct App {}
impl App {
    pub async fn start(chains: HashMap<u64, Chain<&DynProvider>>) -> eyre::Result<()> {
        let block_numbers = chains.values()
            .map(|chain| chain.stream_block_numbers());
        let streams = try_join_all(block_numbers).await?;
        let mut stream = Box::pin(select_all(streams));
        
        let mut solver = Solver::from(chains).await?;
        let executor = TradeExecutor::new();
        
        while let Some(BlockEvent { chain_id, ..}) = stream.next().await {
            let trades = solver.on_block(chain_id).await?;
            executor.execute(trades);
        }

        eyre::bail!("stream of blocks ended unexpectedly");
    }
}
