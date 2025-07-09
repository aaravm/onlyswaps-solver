use crate::eth::ERC20Token::{ERC20TokenErrors, ERC20TokenInstance};
use crate::eth::Router::{RouterErrors, RouterInstance};
use crate::events::BridgeDepositEvent;
use alloy::hex;
use alloy::network::Ethereum;
use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::{DynProvider, Provider};
use superalloy::provider::MultiChainProvider;

pub(crate) struct Chain {
    pub chain_id: U256,
    provider: DynProvider,
    our_address: Address,
    pub router: RouterInstance<DynProvider, Ethereum>,
    token: ERC20TokenInstance<DynProvider, Ethereum>,
}

pub(crate) struct ChainConfig {
    pub chain_id: u64,
    pub our_address: Address,
    pub router_addr: Address,
    pub token_addr: Address,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Transfer {
    pub request_id: FixedBytes<32>,
    pub token: Address,
    pub amount: U256,
    pub src_chain_id: U256,
    pub dest_chain_id: U256,
    pub recipient: Address,
    pub swap_fee: U256,
    pub solver_fee: U256,
    pub nonce: U256,
    pub fulfilled: bool,
}

impl Chain {
    pub fn new(mp: &impl MultiChainProvider<u64>, config: &ChainConfig) -> eyre::Result<Self> {
        let provider = mp
            .get_ethereum_provider(&config.chain_id)
            .ok_or(eyre::eyre!("No provider for chain {}", config.chain_id))?
            .clone();
        let router = RouterInstance::new(config.router_addr, provider.clone());
        let token = ERC20TokenInstance::new(config.token_addr, provider.clone());
        Ok(Chain {
            chain_id: U256::from(config.chain_id),
            our_address: config.our_address,
            provider,
            router,
            token,
        })
    }

    pub async fn fetch_transfer_params(&self, deposit: BridgeDepositEvent) -> eyre::Result<Transfer> {
        let request_id = deposit.request_id.into();
        let transfer = self.router.getTransferParameters(deposit.request_id.into()).call().await?;
        Ok(Transfer {
            request_id,
            token: transfer.token,
            amount: transfer.amount,
            src_chain_id: transfer.srcChainId,
            dest_chain_id: transfer.dstChainId,
            recipient: transfer.recipient,
            swap_fee: transfer.swapFee,
            solver_fee: transfer.solverFee,
            nonce: transfer.nonce,
            fulfilled: transfer.executed,
        })
    }

    pub async fn attempt_token_relay_if_profitable(&self, transfer: Transfer) -> eyre::Result<()> {
        if transfer.dest_chain_id != self.chain_id {
            return Err(eyre::eyre!(
                "transfer request for the wrong chain! expected {} got {}",
                self.chain_id,
                transfer.dest_chain_id
            ));
        }
        if transfer.fulfilled {
            // already been picked up, so we can ignore it
            return Ok(());
        }

        let token_balance = self.token.balanceOf(self.our_address).call().await?;
        if token_balance < transfer.amount {
            println!("not making a trade for request {:?}, as we don't have enough funds", transfer.request_id);
            println!("have {}, need {}", token_balance, transfer.amount);
            return Ok(());
        }
        let native_balance = self.provider.get_balance(self.our_address).await?;
        if !is_profitable(native_balance, token_balance, transfer.solver_fee) {
            println!("not making a trade for request {:?}, as it's unprofitable", transfer.request_id);
            return Ok(());
        }

        let approval = self.token.approve(*self.router.address(), transfer.amount).send().await?;
        approval.get_receipt().await?;
        match self
            .router
            .relayTokens(
                *self.token.address(),
                transfer.recipient,
                transfer.amount,
                transfer.request_id,
                transfer.src_chain_id,
            )
            .send()
            .await
        {
            Ok(pending) => {
                let rx = pending.get_receipt().await?;
                println!("transfer of {} made on chain {}: {}", transfer.amount, transfer.dest_chain_id, rx.transaction_hash);
                Ok(())
            }
            Err(e) => {
                if let Some(err) = e.as_decoded_interface_error::<RouterErrors>() {
                    match err {
                        RouterErrors::AlreadyFulfilled(_) => {
                            println!("request already fulfilled");
                            return Ok(());
                        }
                        RouterErrors::ZeroAmount(_) => eyre::bail!("request zero amount - something went very wrong"),
                        RouterErrors::InvalidTokenOrRecipient(_) => eyre::bail!("invalid token or recipient"),
                        _ => eyre::bail!("failed to decode error"),
                    }
                }
                if let Some(err) = e.as_decoded_interface_error::<ERC20TokenErrors>() {
                    match err {
                        ERC20TokenErrors::ERC20InsufficientAllowance(_) => eyre::bail!("for some strange reason our allowance didn't go through before making the transfer"),
                        ERC20TokenErrors::ERC20InsufficientBalance(_) => {
                            println!("insufficient balance for token transfer of {} - ignoring", transfer.amount);
                            return Ok(());
                        }
                        _ => eyre::bail!("failed to decode error"),
                    }
                }
                    let raw = e.as_revert_data().unwrap_or_default();
                    eyre::bail!("reverted with unknown data: 0x{}", hex::encode(raw))
            }
        }
    }
    pub async fn withdraw_tokens(&self) -> eyre::Result<()> {
        println!("checking funds for {}", self.chain_id);

        let min_balance = U256::from(1_000_000_000);
        let rusd_balance = self.token.balanceOf(self.our_address).call().await?;
        if rusd_balance > min_balance {
            println!("balance {} - not withdrawing tokens for chain_id {}", rusd_balance, &self.chain_id);
            return Ok(());
        }

        let tx = self.token.mint(self.our_address, min_balance).send().await?;
        tx.get_receipt().await?;
        println!("withdrew tokens for chain_id {}", &self.chain_id);
        Ok(())
    }
}

fn is_profitable(native_balance: U256, token_balance: U256, fee: U256) -> bool {
    true
}
