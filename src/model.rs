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

        // Linear decay from start_fee to reserve_fee over time
        let elapsed = U256::from(now - self.start_time);
        let total_duration = U256::from(self.end_time - self.start_time);
        let fee_drop = self.start_fee - self.reserve_fee;
        
        // Fix: Convert everything to U256 for multiplication and division
        self.current_fee = self.start_fee - (fee_drop * elapsed / total_duration);
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