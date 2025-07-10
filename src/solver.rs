use std::collections::HashMap;
use alloy::primitives::U256;
use alloy::providers::DynProvider;
use tonic::async_trait;
use crate::chain::Chain;
use crate::eth::IRouter::TransferParams;
use crate::model::{ChainState, Trade};

#[async_trait]
pub(crate) trait ChainStateProvider {
    async fn fetch_state(&self) -> eyre::Result<ChainState>;
}

pub(crate) struct Solver<CSP> {
    states: HashMap<u64, ChainState>,
    chains: HashMap<u64, CSP>,
}

impl<CSP> Default for Solver<CSP> {
    fn default() -> Self {
        Self {
            states: HashMap::default(),
            chains: HashMap::default(),
        }
    }
}
impl<CSP: ChainStateProvider> Solver<CSP> {
    fn new(initial_states: HashMap<u64, ChainState>, chains: HashMap<u64, CSP>) -> Self {
        Self {
            states: initial_states,
            chains,
        }
    }
    
    pub async fn from(chains: HashMap<u64, CSP>) -> eyre::Result<Self> {
        let mut states: HashMap<u64, ChainState> = HashMap::new();

        // set the initial state for each
        for (chain_id, chain) in &chains {
            states.insert(*chain_id, chain.fetch_state().await?);
        } 
        
        Ok(Self {
            states,
            chains
        })
    }

    pub async fn extend(&mut self, chain_id: u64, state_provider: CSP) -> eyre::Result<&mut Solver<CSP>> {
        let state = state_provider.fetch_state().await?;
        self.chains.insert(chain_id, state_provider);
        self.states.insert(chain_id, state);
        Ok(self)
    }
    pub async fn on_block(&mut self, chain_id: u64) -> eyre::Result<Vec<Trade>> {
        let chain = self.chains.get(&chain_id).expect("somehow got event for a non-existent chain");
        let updated_state = chain.fetch_state().await?;
        self.states.insert(chain_id, updated_state);
        Ok(calculate_trades(&self.states))
    }
}


fn calculate_trades(states: &HashMap<u64, ChainState>) -> Vec<Trade> {
    let mut trades = Vec::new();
    for (chain_id, state) in states {
        if state.transfers.is_empty() {
            continue;
        }
        for transfer in &state.transfers {
            let TransferParams {
                amount,
                dstChainId: dst_chain_id,
                ..
            } = transfer.params;
            if let Some(other_state) = states.get(&normalise_chain_id(dst_chain_id)) {
                if other_state.token_balance >= amount {
                    let trade = Trade {
                        chain_id: *chain_id,
                        request_id: transfer.request_id,
                        amount,
                    };
                    trades.push(trade);
                }
            }
        }
    }
    trades
}

fn normalise_chain_id(chain_id: U256) -> u64 {
    chain_id.as_limbs()[0]
}

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;
    use tonic::async_trait;
    use crate::model::ChainState;
    use crate::solver::{ChainStateProvider, Solver};

    #[tokio::test]
    async fn e2e_test() {
        let chain_id = 1;
        let starting_state = ChainState {
            native_balance: U256::from(1),
            token_balance: U256::from(1),
            transfers: Vec::default(),
        };
        let next_state = ChainState {
            native_balance: U256::from(0),
            token_balance: U256::from(1),
            transfers: Vec::default(),
        };
        let chain = StubbedChain { starting_state, next_state };
        let mut solver = Solver::default();
        let solver = solver.extend(chain_id, chain).await.unwrap();
        let trades = solver.on_block(chain_id).await.unwrap();
        assert_eq!(trades.len(), 0);
    }

    struct StubbedChain {
        starting_state: ChainState,
        next_state: ChainState,
    }

    #[async_trait]
    impl ChainStateProvider for StubbedChain {
        async fn fetch_state(&self) -> eyre::Result<ChainState> {
            Ok(self.next_state.clone())
        }
    }
}
