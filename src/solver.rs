use crate::eth::IRouter::SwapRequestParameters;
use crate::model::{ChainState, RequestId, Trade, Transfer, DutchAuction};
use crate::util::normalise_chain_id;
use alloy::primitives::U256;
use async_trait::async_trait;
use moka::sync::Cache;
use std::collections::HashMap;

#[async_trait]
pub(crate) trait ChainStateProvider {
    async fn fetch_state(&self) -> eyre::Result<ChainState>;
}

pub(crate) struct Solver<'a, CSP> {
    states: HashMap<u64, ChainState>,
    chains: &'a HashMap<u64, CSP>,
    initial_transfers: HashMap<u64, Vec<RequestId>>, // Track transfers that existed at startup
    demo_mode: bool, // Allow processing of pre-fulfilled transfers for demo purposes
}
impl<'a, CSP: ChainStateProvider> Solver<'a, CSP> {
    pub async fn from(chains: &'a HashMap<u64, CSP>) -> eyre::Result<Self> {
        let mut states: HashMap<u64, ChainState> = HashMap::new();
        let mut initial_transfers: HashMap<u64, Vec<RequestId>> = HashMap::new();

        // fetch the initial state for each chain before we listen for blocks
        for (chain_id, chain) in chains {
            let state = chain.fetch_state().await?;
            
            // Record initial transfers to distinguish from new ones
            let initial_transfer_ids = state.transfers.iter()
                .map(|t| t.request_id)
                .collect();
            initial_transfers.insert(*chain_id, initial_transfer_ids);
            
            states.insert(*chain_id, state);
        }

        Ok(Self { states, chains, initial_transfers, demo_mode: true })
    }
    pub async fn fetch_state(&mut self, chain_id: u64, in_flight: &Cache<RequestId, ()>) -> eyre::Result<Vec<Trade>> {
        let chain = self.chains.get(&chain_id).expect("somehow got event for a non-existent chain");
        let mut updated_state = chain.fetch_state().await?;

        // Preserve existing auctions from the old state
        if let Some(existing_state) = self.states.get(&chain_id) {
            updated_state.active_auctions = existing_state.active_auctions.clone();
        }

        // Insert the updated state FIRST
        self.states.insert(chain_id, updated_state);
        
        // Start auctions for new transfers
        self.start_auctions_for_new_transfers(chain_id);
        
        // Calculate trades for all known chains
        let mut all_trades = Vec::new();
        
        // Check all configured chains
        for &chain_id in self.chains.keys() {
            let mut chain_trades = self.calculate_trades_internal(chain_id, in_flight);
            all_trades.append(&mut chain_trades);
        }
        
        Ok(all_trades)
    }
    // Helper method to check if a transfer is new (appeared after startup)
    fn is_new_transfer(&self, chain_id: u64, request_id: &RequestId) -> bool {
        if let Some(initial_transfers) = self.initial_transfers.get(&chain_id) {
            !initial_transfers.contains(request_id)
        } else {
            true // If we don't have initial state, consider it new
        }
    }

    // Toggle demo mode to allow processing pre-fulfilled transfers
    pub fn set_demo_mode(&mut self, enabled: bool) {
        self.demo_mode = enabled;
        println!("🎮 Demo mode {}", if enabled { "enabled" } else { "disabled" });
    }

    // Fixed method - creates auctions on destination chains, not source chains
    fn start_auctions_for_new_transfers(&mut self, chain_id: u64) {
        // Get the transfers from this chain (source chain)
        let transfers = if let Some(state) = self.states.get(&chain_id) {
            state.transfers.clone()
        } else {
            return;
        };
        
        // For each transfer, create auction on the DESTINATION chain
        for transfer in &transfers {
            let dest_chain_id = normalise_chain_id(transfer.params.dstChainId);
            
            // Create auction on destination chain, not source chain
            if let Some(dest_state) = self.states.get_mut(&dest_chain_id) {
                // Check if this is a new transfer (auction doesn't exist yet)
                if !dest_state.active_auctions.contains_key(&transfer.request_id) {
                    // Use slippage-based auction: solverFee is now slippage tolerance
                    let expected_blocks = 60; // Expected number of blocks for the auction
                    let auction = DutchAuction::new_slippage_based(
                        transfer.params.amountOut, // Token amount
                        transfer.params.solverFee, // Slippage tolerance (repurposed)
                        expected_blocks
                    );
                    println!("🚀 Started slippage-based Dutch auction for request {:?} on DESTINATION chain {}", 
                        transfer.request_id, dest_chain_id);
                    println!("   Amount: {}, Slippage: {}", transfer.params.amountOut, transfer.params.solverFee);
                    println!("   Start price: {}, Reserve price (minAllowedCost): {}", 
                        auction.start_fee, auction.reserve_fee);
                    
                    dest_state.active_auctions.insert(transfer.request_id, auction);
                }
            }
        }
    }
    // New internal method that works with self.states directly
    fn calculate_trades_internal(&mut self, chain_id: u64, in_flight: &Cache<RequestId, ()>) -> Vec<Trade> {
        let mut trades = Vec::new();
        
        println!("🔄 Checking chain {} for trades", chain_id);
        
        // Get transfers without cloning states
        let transfers = self.states
            .get(&chain_id)
            .expect("somehow we got a block from a chain that doesn't have a state")
            .transfers
            .clone(); // Only clone transfers

        println!("📋 Found {} transfers on chain {}", transfers.len(), chain_id);

        for transfer in &transfers {
            if in_flight.contains_key(&transfer.request_id) {
                println!("⏭️ Skipping transfer {:?} - already in flight", transfer.request_id);
                continue;
            }
            
            // Call solve with direct access to self.states (no cloning!)
            self.solve_internal(&transfer, &mut trades);
        }

        println!("🎯 Generated {} trades from chain {}", trades.len(), chain_id);
        trades
    }

    // New solve method that works with self.states directly
    fn solve_internal(&mut self, transfer_request: &Transfer, trades: &mut Vec<Trade>) {
        let SwapRequestParameters {
            dstChainId,
            tokenOut,
            amountOut,
            solverFee,
            executed,
            ..
        } = transfer_request.params;

        println!("🔍 Processing transfer {:?} for destination chain {}", transfer_request.request_id, normalise_chain_id(dstChainId));

        // Check if this is a new transfer before getting mutable borrow
        let is_new_transfer = self.is_new_transfer(normalise_chain_id(transfer_request.params.srcChainId), &transfer_request.request_id);

        let dest_state = match self.states.get_mut(&normalise_chain_id(dstChainId)) {
            None => {
                println!("❌ Destination chain {} not found in states", normalise_chain_id(dstChainId));
                return;
            }
            Some(state) => {
                println!("✅ Found destination chain {} with {} auctions", normalise_chain_id(dstChainId), state.active_auctions.len());
                state
            }
        };

        if executed {
            println!("❌ Transfer already executed, returning");
            return;
        }

        if dest_state.already_fulfilled.contains(&transfer_request.request_id) {
            println!("⚠️ Transfer already fulfilled on blockchain (new_transfer: {}, demo_mode: {})", is_new_transfer, self.demo_mode);
            
            if is_new_transfer && !self.demo_mode {
                println!("❌ New transfer that was quickly fulfilled, skipping");
                return;
            } else if !is_new_transfer && self.demo_mode {
                println!("🔄 Pre-existing fulfilled transfer, allowing auction for demo purposes");
            } else if !self.demo_mode {
                println!("❌ Transfer already fulfilled, skipping (demo mode disabled)");
                return;
            }
        }

        if dest_state.native_balance == U256::from(0) {
            println!("❌ No native balance, returning");
            return;
        }

        if dest_state.token_balance < amountOut {
            println!("❌ Insufficient token balance: {} < {}, returning", dest_state.token_balance, amountOut);
            return;
        }

        // Validate slippage tolerance (solverFee is now slippage in basis points)
        // Slippage should be between 0 and 10000 (0% to 100%)
        if solverFee > U256::from(10000) {
            println!("❌ Slippage tolerance too high: {} bps (max 10000), returning", solverFee);
            return;
        }

        if tokenOut != dest_state.token_addr {
            println!("❌ Token mismatch: {} != {}, returning", tokenOut, dest_state.token_addr);
            return;
        }

        // Check each validation condition with debug output
        println!("🔍 Validating transfer conditions:");
        println!("   executed: {}", executed);
        println!("   already_fulfilled: {}", dest_state.already_fulfilled.contains(&transfer_request.request_id));
        println!("   native_balance: {}", dest_state.native_balance);
        println!("   token_balance: {} (needed: {})", dest_state.token_balance, amountOut);
        println!("   slippage_tolerance_bps: {}", solverFee);
        println!("   tokenOut matches: {}", tokenOut == dest_state.token_addr);

        // Slippage-based Dutch Auction Logic
        let (current_price, should_execute) = if let Some(auction) = dest_state.active_auctions.get_mut(&transfer_request.request_id) {
            println!("🎯 Found slippage-based auction for {:?} on destination chain!", transfer_request.request_id);
            let current_price = auction.update_current_fee();
            
            // New execution threshold: 2 * minAllowedCost (reserve_fee is minAllowedCost)
            let min_allowed_cost = auction.reserve_fee;
            let execution_threshold = min_allowed_cost * U256::from(2);
            let should_execute = current_price <= execution_threshold;
            
            println!("💰 Slippage Auction {:?} - Current price: {}, MinAllowedCost: {}, Threshold (2x): {}, Execute: {}", 
                transfer_request.request_id, current_price, min_allowed_cost, execution_threshold, should_execute);
            
            if auction.is_expired() {
                println!("⏰ Auction {:?} expired, executing at minAllowedCost", transfer_request.request_id);
                (auction.reserve_fee, true)
            } else {
                (current_price, should_execute)
            }
        } else {
            println!("❌ No auction found for {:?} on destination chain {}", transfer_request.request_id, normalise_chain_id(dstChainId));
            // Fallback: treat solverFee as slippage and calculate minAllowedCost directly
            let slippage_bps = U256::from(10000);
            let min_allowed_cost = amountOut * (slippage_bps - solverFee) / slippage_bps;
            println!("📈 Using fallback slippage calculation - Amount: {}, Slippage: {}, MinAllowedCost: {}", 
                amountOut, solverFee, min_allowed_cost);
            (min_allowed_cost, true)
        };

        if !should_execute {
            println!("❌ Not executing trade {:?} - price too high", transfer_request.request_id);
            return;
        }

        println!("✅ Executing trade {:?} at price {}", transfer_request.request_id, current_price);
        
        dest_state.token_balance -= amountOut;
        
        let trade = Trade {
            request_id: transfer_request.request_id,
            token_addr: transfer_request.params.tokenOut,
            src_chain_id: transfer_request.params.srcChainId,
            dest_chain_id: transfer_request.params.dstChainId,
            recipient_addr: transfer_request.params.recipient,
            swap_amount: amountOut,
            auction_price: current_price,
        };
        
        trades.push(trade);
        dest_state.active_auctions.remove(&transfer_request.request_id);
    }

    // Keep the old methods for backward compatibility with tests
    fn start_new_auctions(&mut self, chain_id: u64) {
        if let Some(state) = self.states.get_mut(&chain_id) {
            let transfers = state.transfers.clone();
            
            for transfer in &transfers {
                if !state.active_auctions.contains_key(&transfer.request_id) {
                    // Keep old logic for backward compatibility in tests
                    let auction = DutchAuction::new(
                        transfer.params.solverFee,
                        60,
                        3.0
                    );
                    println!("🚀 Started Dutch auction for request {:?} on chain {}", 
                        transfer.request_id, chain_id);
                    println!("   Start price: {}, Reserve price: {}", 
                        auction.start_fee, auction.reserve_fee);
                    
                    state.active_auctions.insert(transfer.request_id, auction);
                }
            }
        }
    }
}

// Keep the old functions for tests (add active_auctions field to ChainState in tests)
fn calculate_trades(chain_id: u64, states: &HashMap<u64, ChainState>, in_flight: &Cache<RequestId, ()>) -> Vec<Trade> {
    let mut trades = Vec::new();
    let mut solve_states = states.clone();
    
    let transfers = states
        .get(&chain_id)
        .expect("somehow we got a block from a chain that doesn't have a state")
        .transfers
        .clone();

    for transfer in &transfers {
        if in_flight.contains_key(&transfer.request_id) {
            continue;
        }
        solve(&transfer, &mut trades, &mut solve_states);
    }

    trades
}

// Keep old solve function for tests
fn solve(transfer_request: &Transfer, trades: &mut Vec<Trade>, states: &mut HashMap<u64, ChainState>) {
    // ... existing solve logic without auction support for tests
    let SwapRequestParameters {
        dstChainId,
        tokenOut,
        amountOut,
        solverFee,
        executed,
        ..
    } = transfer_request.params;

    let dest_state = match states.get_mut(&normalise_chain_id(dstChainId)) {
        None => return,
        Some(state) => state,
    };

    if executed || dest_state.already_fulfilled.contains(&transfer_request.request_id) ||
       dest_state.native_balance == U256::from(0) || dest_state.token_balance < amountOut ||
       solverFee < U256::from(1) || tokenOut != dest_state.token_addr {
        return;
    }

    dest_state.token_balance -= amountOut;
    
    let trade = Trade {
        request_id: transfer_request.request_id,
        token_addr: transfer_request.params.tokenOut,
        src_chain_id: transfer_request.params.srcChainId,
        dest_chain_id: transfer_request.params.dstChainId,
        recipient_addr: transfer_request.params.recipient,
        swap_amount: amountOut,
        auction_price: solverFee, // Default for tests
    };
    
    trades.push(trade);
}

#[cfg(test)]
mod tests {
    use crate::eth::IRouter::SwapRequestParameters;
    use crate::model::{ChainState, Trade, Transfer};
    use crate::solver::{ChainStateProvider, Solver, calculate_trades};
    use crate::util::test::{generate_address, generate_request_id};
    use alloy::primitives::{Address, U256, address};
    use async_trait::async_trait;
    use moka::sync::Cache;
    use speculoos::assert_that;
    use speculoos::vec::VecAssertions;
    use std::collections::HashMap;

    static USER_ADDR: Address = address!("0xdeadbeef6964af9d7eed9e03e53415d37aa96045");
    static TOKEN_ADDR: Address = address!("0xd8da6bf26964af9d7eed9e03e53415d37aa96045");

    #[tokio::test]
    async fn transfers_created_through_solver_create_trades() {
        // given
        let chain_id = 1;
        let transfer_params = create_transfer_params(USER_ADDR, 1, 2, 100);
        let chain_one_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1),
            token_balance: U256::from(1),
            transfers: vec![transfer_params.clone()],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let chain_two_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(100),
            token_balance: U256::from(1000),
            transfers: Vec::default(),
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let chain_one = StubbedChain { state: chain_one_state };
        let chain_two = StubbedChain { state: chain_two_state };
        let networks = HashMap::from([(1, chain_one), (2, chain_two)]);

        // when
        let mut solver = Solver::from(&networks).await.unwrap();
        let trades = solver.fetch_state(chain_id, &Cache::new(1)).await.unwrap();

        // then
        let expected_output_amount = transfer_params.params.amountOut;
        let expected_trade = Trade {
            request_id: transfer_params.request_id,
            token_addr: transfer_params.params.tokenOut,
            src_chain_id: transfer_params.params.srcChainId,
            dest_chain_id: transfer_params.params.dstChainId,
            recipient_addr: transfer_params.params.recipient,
            swap_amount: expected_output_amount,
            auction_price: U256::from(1), // Default solver fee from test
        };
        assert_that!(trades).has_length(1);
        assert_that!(trades[0]).is_equal_to(expected_trade);
    }

    #[test]
    fn multiple_transfers_create_multiple_trades() {
        // given
        // both transfers use 100
        let transfer_params = create_transfer_params(USER_ADDR, 1, 2, 100);
        let transfer_params_2 = create_transfer_params(USER_ADDR, 1, 2, 100);

        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(0),
            transfers: vec![transfer_params, transfer_params_2],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        // on dst_chain, we only have enough balance to cover one tx
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1000),
            token_balance: U256::from(200),
            transfers: vec![],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &Cache::new(1));

        // then
        assert_that!(trades).has_length(2);
    }

    #[test]
    fn transfers_across_multiple_chains_only_create_trades_for_src_chain() {
        // given
        // both transfers use 100
        let transfer_params = create_transfer_params(USER_ADDR, 1, 2, 100);
        let transfer_params_2 = create_transfer_params(USER_ADDR, 1, 2, 100);

        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1000),
            token_balance: U256::from(100),
            transfers: vec![transfer_params],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        // on dst_chain, we only have enough balance to cover one tx
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1000),
            token_balance: U256::from(200),
            transfers: vec![transfer_params_2],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &Cache::new(1));

        // then
        assert_that!(trades).has_length(1);
    }

    #[test]
    fn no_transfers_creates_no_trades() {
        // given
        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(0),
            transfers: vec![],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(1000),
            transfers: vec![],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &Cache::new(1));

        // then
        assert_that!(trades).has_length(0);
    }

    #[test]
    fn no_native_currency_on_dest_chain_doesnt_trade() {
        // given
        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(0),
            transfers: vec![create_transfer_params(USER_ADDR, 1, 2, 100)],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(1000),
            transfers: vec![],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &Cache::new(1));

        // then
        assert_that!(trades).has_length(0);
    }

    #[test]
    fn no_token_balance_doesnt_trade() {
        // given
        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(0),
            transfers: vec![create_transfer_params(USER_ADDR, 1, 2, 100)],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1000),
            token_balance: U256::from(0),
            transfers: vec![],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &Cache::new(1));

        // then
        assert_that!(trades).has_length(0);
    }

    #[test]
    fn already_executed_doesnt_create_tx() {
        // given
        let mut transfer_params = create_transfer_params(USER_ADDR, 1, 2, 100);
        transfer_params.params.executed = true;

        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(0),
            transfers: vec![transfer_params],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1000),
            token_balance: U256::from(1000),
            transfers: vec![],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &Cache::new(1));

        // then
        assert_that!(trades).has_length(0);
    }

    #[test]
    fn no_fee_gives_no_trade() {
        // given
        let mut transfer_params = create_transfer_params(USER_ADDR, 1, 2, 100);
        transfer_params.params.solverFee = U256::from(0);

        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(0),
            transfers: vec![transfer_params],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1000),
            token_balance: U256::from(1000),
            transfers: vec![],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &Cache::new(1));

        // then
        assert_that!(trades).has_length(0);
    }

    #[test]
    fn invalid_token_addr_gives_no_trade() {
        // given
        let mut transfer_params = create_transfer_params(USER_ADDR, 1, 2, 100);
        transfer_params.params.tokenOut = generate_address();

        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(0),
            transfers: vec![transfer_params],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1000),
            token_balance: U256::from(1000),
            transfers: vec![],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &Cache::new(1));

        // then
        assert_that!(trades).has_length(0);
    }

    #[test]
    fn subsequent_calls_dont_use_same_balance() {
        // given
        // both transfers use 100
        let transfer_params = create_transfer_params(USER_ADDR, 1, 2, 100);
        let transfer_params_2 = create_transfer_params(USER_ADDR, 1, 2, 100);

        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(0),
            transfers: vec![transfer_params, transfer_params_2],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        // on dst_chain, we only have enough balance to cover one tx
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1000),
            token_balance: U256::from(150),
            transfers: vec![],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &Cache::new(1));

        // then
        assert_that!(trades).has_length(1);
    }

    #[test]
    fn transfers_that_have_already_been_fulfilled_dont_make_trades() {
        // given
        // both transfers use 100
        let transfer_params = create_transfer_params(USER_ADDR, 1, 2, 100);

        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(0),
            transfers: vec![transfer_params.clone()],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        // on dst_chain, we only have enough balance to cover one tx
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1000),
            token_balance: U256::from(150),
            transfers: vec![],
            already_fulfilled: vec![transfer_params.request_id],
            active_auctions: HashMap::new(),
        };
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &Cache::new(1));

        // then
        assert_that!(trades).has_length(0);
    }

    #[test]
    fn transfer_that_exist_in_cache_dont_make_trades() {
        // given
        // transfer use 100
        let transfer_params = create_transfer_params(USER_ADDR, 1, 2, 100);

        let src_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(0),
            token_balance: U256::from(0),
            transfers: vec![transfer_params.clone()],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        let dst_chain_state = ChainState {
            token_addr: TOKEN_ADDR,
            native_balance: U256::from(1000),
            token_balance: U256::from(200),
            transfers: vec![],
            already_fulfilled: vec![],
            active_auctions: HashMap::new(),
        };
        // we create a cache that already has the request_id in it
        let cache = Cache::new(1);
        let id = transfer_params.clone().request_id;
        cache.insert(id, ());
        let state = HashMap::from([(1, src_chain_state), (2, dst_chain_state)]);

        // when
        let trades = calculate_trades(1, &state, &cache);

        // then
        assert_that!(trades).has_length(0);
    }

    fn create_transfer_params(sender: Address, src_chain_id: u64, dest_chain_id: u64, amount: u64) -> Transfer {
        Transfer {
            request_id: generate_request_id(),
            params: SwapRequestParameters {
                srcChainId: U256::from(src_chain_id),
                dstChainId: U256::from(dest_chain_id),
                sender,
                recipient: sender,
                tokenIn: TOKEN_ADDR,
                tokenOut: TOKEN_ADDR,
                amountOut: U256::from(amount),
                verificationFee: U256::from(2),
                solverFee: U256::from(5000), // 50% slippage to make execution more likely in tests
                nonce: U256::from(100),
                executed: false,
                requestedAt: U256::from(12345),
            },
        }
    }
    struct StubbedChain {
        state: ChainState,
    }

    #[async_trait]
    impl ChainStateProvider for StubbedChain {
        async fn fetch_state(&self) -> eyre::Result<ChainState> {
            Ok(self.state.clone())
        }
    }
}
