use crate::executor::TradeExecutor;
use crate::model::{BlockEvent, RequestId};
use crate::network::Network;
use crate::solver::Solver;
use crate::config::{ConfigFile, SolverConfig};
use alloy::providers::DynProvider;
use futures::StreamExt;
use futures::future::try_join_all;
use futures::stream::select_all;
use moka::sync::Cache;
use std::collections::HashMap;
use std::time::Duration;

pub struct App {}
impl App {
    pub async fn start(networks: HashMap<u64, Network<DynProvider>>, config: &ConfigFile, solver_id: u8) -> eyre::Result<()> {
        let block_numbers = networks.values().map(|network| network.stream_block_numbers());
        let streams = try_join_all(block_numbers).await?;
        let mut stream = Box::pin(select_all(streams));

        // Get solver configuration or use defaults
        let (threshold_multiplier, solver_name) = if let Some(solver_config) = &config.solver_config {
            (solver_config.threshold_multiplier, format!("{}#{}", solver_config.solver_name, solver_id))
        } else {
            // Configure solver with competitive defaults - INVERTED for correct competitive behavior
            match solver_id {
                1 => (1.1, format!("AggressiveSolver#{}", solver_id)),    // Most aggressive - 10% down (90% of start price)
                2 => (1.5, format!("ModerateSolver#{}", solver_id)),      // Moderate - 33% down (67% of start price)  
                3 => (2.0, format!("ConservativeSolver#{}", solver_id)),  // Conservative - 50% down (50% of start price)
                _ => panic!("Invalid solver_id: {}. Must be 1, 2, or 3", solver_id),
            }
        };

        let mut solver = Solver::from(&networks, threshold_multiplier, solver_name).await?;
        let executor = TradeExecutor::new(&networks);

        // we pull new chain state every block, so inflight requests may not have been
        // completed yet, so we don't want to attempt to execute them again and waste gas.
        // if they're still there after 30s we can reattempt
        let mut inflight_requests: Cache<RequestId, ()> = Cache::builder().max_capacity(1000).time_to_live(Duration::from_secs(30)).build();

        while let Some(BlockEvent { chain_id, .. }) = stream.next().await {
            // Add solver-specific delay to simulate real-world processing differences
            let delay_ms = match solver_id {
                1 => 0,   // AggressiveSolver: fastest processing (immediate)
                2 => 100, // ModerateSolver: moderate delay  
                3 => 250, // ConservativeSolver: significant delay
                _ => 500,
            };
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            
            let trades = solver.fetch_state(chain_id, &inflight_requests).await?;
            if !trades.is_empty() {
                println!("executing {} trades from chain {}", trades.len(), chain_id);
                executor.execute(trades, &mut inflight_requests).await;
                
                // ✅ IMMEDIATE STATE REFRESH: Update all solver states after execution
                // This helps other solvers quickly detect completed trades
                for &refresh_chain in networks.keys() {
                    let _ = solver.refresh_chain_state(refresh_chain).await;
                }
            }
        }

        eyre::bail!("stream of blocks ended unexpectedly");
    }
}
