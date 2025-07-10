use alloy::primitives::U256;
use crate::eth::IRouter::TransferParams;

#[derive(Debug, Clone)]
pub struct ChainState {
    pub native_balance: U256,
    pub token_balance: U256,
    pub transfers: Vec<Transfer>,
}

#[derive(Debug, Clone)]
pub struct Transfer {
    pub request_id: [u8; 32],
    pub params: TransferParams,
}

#[derive(Debug)]
pub struct Trade {
    pub chain_id: u64,
    pub request_id: [u8; 32],
    pub amount: U256,
}