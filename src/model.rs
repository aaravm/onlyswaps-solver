use crate::eth::IRouter::SwapRequestParameters;
use alloy::primitives::{Address, U256};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

pub type RequestId = [u8; 32];

#[derive(Debug, Clone)]
pub struct DutchAuction {
    pub start_time: u64,
    pub end_time: u64,
    pub start_fee: U256,
    pub reserve_fee: U256,
    pub current_fee: U256,
}

impl DutchAuction {
    pub fn new(base_fee: U256, duration_seconds: u64, multiplier: f64) -> Self {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let start_fee = U256::from((base_fee.to::<u64>() as f64 * multiplier) as u64);
        
        Self {
            start_time,
            end_time: start_time + duration_seconds,
            start_fee,
            reserve_fee: base_fee,
            current_fee: start_fee,
        }
    }

    // New constructor for slippage-based auctions
    pub fn new_slippage_based(amount: U256, slippage: U256, expected_blocks: u64) -> Self {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        println!("Initial amount is: {}", amount);
        // Calculate minAllowedCost = (1 - slippage) * amount
        // Assuming slippage is in basis points (e.g., 100 = 1%)
        let slippage_bps = U256::from(10000); // 100%
        let min_allowed_cost = amount * (slippage_bps - slippage) / slippage_bps;
        
        // Start price = 3 * minAllowedCost
        let start_fee = min_allowed_cost * U256::from(3);
        
        Self {
            start_time,
            end_time: start_time + expected_blocks, // Using blocks instead of seconds
            start_fee,
            reserve_fee: min_allowed_cost, // Reserve price is minAllowedCost
            current_fee: start_fee,
        }
    }

    pub fn update_current_fee(&mut self) -> U256 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        if now >= self.end_time {
            self.current_fee = self.reserve_fee;
            return self.current_fee;
        }

        if now <= self.start_time {
            self.current_fee = self.start_fee;
            return self.current_fee;
        }

        // Linear decay from start_fee to reserve_fee over time (deterministic)
        let elapsed = U256::from(now - self.start_time);
        let total_duration = U256::from(self.end_time - self.start_time);
        let fee_drop = self.start_fee - self.reserve_fee;

        self.current_fee = self.start_fee - (fee_drop * elapsed / total_duration);
        self.current_fee
    }

    // New method that accepts randomness as a parameter (for drand integration)
    pub fn update_current_fee_with_randomness(&mut self, randomness: f64) -> U256 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        if now >= self.end_time {
            self.current_fee = self.reserve_fee;
            return self.current_fee;
        }

        if now <= self.start_time {
            self.current_fee = self.start_fee;
            return self.current_fee;
        }

        // Linear decay with randomness applied
        let elapsed = U256::from(now - self.start_time);
        let total_duration = U256::from(self.end_time - self.start_time);
        let base_fee_drop = self.start_fee - self.reserve_fee;

        // Apply randomness: scale by 0 to 2x based on normalized random value [0,1)
        let rnd_scaled_u64 = ((randomness * 2.0 * 1_000_000.0).round() as u64).min(2_000_000);
        let rnd_u256 = U256::from(rnd_scaled_u64);

        let randomized_fee_drop = base_fee_drop * rnd_u256 / U256::from(10_000_000u64);

        self.current_fee = self.start_fee - (randomized_fee_drop * elapsed / total_duration);
        
        println!(" Time-based randomness: {:.6}, Scaled: {}, Fee drop: {}", 
                randomness, rnd_scaled_u64, randomized_fee_drop);
        
        self.current_fee
    }

    // New method for block-based price updates with custom step size
    pub fn update_current_fee_by_blocks(&mut self, current_block: u64) -> U256 {
        if current_block >= self.end_time {
            self.current_fee = self.reserve_fee;
            return self.current_fee;
        }

        let start_block = self.start_time; // Repurposing start_time as start_block
        if current_block <= start_block {
            self.current_fee = self.start_fee;
            return self.current_fee;
        }

        // Calculate price decrease per block
        // Price decreases by (2 * minAllowedCost / expected_blocks) per block
        let min_allowed_cost = self.reserve_fee;
        let total_blocks = self.end_time - start_block;
        let price_decrease_per_block = (min_allowed_cost * U256::from(2)) / U256::from(total_blocks);
        
        let blocks_elapsed = current_block - start_block;
        let total_decrease = price_decrease_per_block * U256::from(blocks_elapsed);
        
        if total_decrease >= self.start_fee - self.reserve_fee {
            self.current_fee = self.reserve_fee;
        } else {
            self.current_fee = self.start_fee - total_decrease;
        }
        
        self.current_fee
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        now >= self.end_time
    }

    pub fn is_profitable(&self, min_profit_threshold: U256) -> bool {
        self.current_fee >= min_profit_threshold
    }
}

#[derive(Debug, Clone)]
pub struct ChainState {
    pub token_addr: Address, // this is kinda yuck, but simplest way to support it for now
    pub native_balance: U256,
    pub token_balance: U256,
    pub transfers: Vec<Transfer>,
    pub already_fulfilled: Vec<RequestId>,
    pub active_auctions: HashMap<RequestId, DutchAuction>, // Add this field
}

// Add auction field to Transfer and required derives
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Transfer {
    pub request_id: RequestId,
    pub params: SwapRequestParameters,
}

// Add auction_price field to Trade
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Trade {
    pub token_addr: Address,
    pub src_chain_id: U256,
    pub dest_chain_id: U256,
    pub recipient_addr: Address,
    pub request_id: RequestId,
    pub swap_amount: U256,
    pub auction_price: U256, // Add this field
}

impl From<&Transfer> for Trade {
    fn from(transfer: &Transfer) -> Self {
        Trade {
            token_addr: transfer.params.tokenOut,
            src_chain_id: transfer.params.srcChainId,
            dest_chain_id: transfer.params.dstChainId,
            recipient_addr: transfer.params.recipient,
            request_id: transfer.request_id,
            swap_amount: transfer.params.amountOut,
            auction_price: transfer.params.solverFee, // Default to base fee
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct BlockEvent {
    pub chain_id: u64,
    pub block_number: u64,
}