use crate::ERC20FaucetToken;
use alloy::primitives::U256;
use alloy::providers::Provider;
use alloy::signers::Signer;
use crate::ERC20FaucetToken::ERC20FaucetTokenInstance;

struct Faucet<S, P: Provider> {
    contract: ERC20FaucetTokenInstance<P>,
    signer: S,
}

impl<S: Signer, P: Provider> Faucet<S, P> {
    fn new(token_address: String, signer: S, provider: P) -> eyre::Result<Self> {
        let contract =
            ERC20FaucetToken::new(token_address.parse()?, provider);
        Ok(Self {
            contract,
            signer,
        })
    }
    async fn withdraw(&self) -> eyre::Result<()> {
        let our_address = self.signer.address();

        let rusd_balance = self.contract.balanceOf(our_address).call().await?;
        if rusd_balance == U256::from(0) {
            println!("withdrawing some tokens");
            let tx = self.contract.mint().send().await?;
            let receipt = tx.get_receipt().await?;
            println!("withdrew tokens: {}", receipt.transaction_hash);
        } else {
            println!("balance {} - not withdrawing tokens", rusd_balance);
        }
        Ok(())
    }
}
