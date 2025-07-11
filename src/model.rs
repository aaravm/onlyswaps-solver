use crate::eth::IRouter::TransferParams;
use alloy::primitives::{Address, U256};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChainState {
    pub token_addr: Address, // this is kinda yuck, but simplest way to support it for now
    pub native_balance: U256,
    pub token_balance: U256,
    pub transfers: Vec<Transfer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Transfer {
    pub request_id: [u8; 32],
    pub params: TransferParams,
}

impl From<&Transfer> for Trade {
    fn from(transfer: &Transfer) -> Self {
        Trade {
            token_addr: transfer.params.token,
            src_chain_id: transfer.params.srcChainId,
            dest_chain_id: transfer.params.dstChainId,
            recipient_addr: transfer.params.recipient,
            request_id: transfer.request_id,
            amount: transfer.params.amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Trade {
    pub token_addr: Address,
    pub src_chain_id: U256,
    pub dest_chain_id: U256,
    pub recipient_addr: Address,
    pub request_id: [u8; 32],
    pub amount: U256,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct BlockEvent {
    pub chain_id: u64,
    pub block_number: u64,
}
