use alloy::primitives::Address;
use bytes::Bytes;
use eyre::eyre;
use omnievent::proto_types::omni_event_service_client::OmniEventServiceClient;
use omnievent::proto_types::{EventOccurrence, RegisterNewEventRequest, event_data};
use std::array::TryFromSliceError;
use std::sync::Arc;
use superalloy::provider::MultiChainProvider;
use crate::eth::Router::RouterInstance;

pub(crate) trait EventHandler {
    fn register(&mut self, client: &mut OmniEventServiceClient<tonic::transport::channel::Channel>) -> impl Future<Output=eyre::Result<()>>;
    fn handle(&self, e: &EventOccurrence) -> impl Future<Output=eyre::Result<()>>;
}

pub(crate) struct BridgeDepositHandler<MP> {
    chain_id: u64,
    contract_addr: Address,
    pub events: Vec<Bytes>,
    multi_provider: Arc<MP>,
}

impl<MP> BridgeDepositHandler<MP>
where
    MP: MultiChainProvider<u64>,
{
    pub fn new(chain_id: u64, contract_addr: Address, multi_provider: Arc<MP>) -> eyre::Result<Self> {
        Ok(Self {
            chain_id,
            contract_addr,
            multi_provider,
            events: Vec::default(),
        })
    }
}

impl<MP> EventHandler for BridgeDepositHandler<MP>
where
    MP: MultiChainProvider<u64>,
{
    async fn register(&mut self, client: &mut OmniEventServiceClient<tonic::transport::channel::Channel>) -> eyre::Result<()> {
        let request = RegisterNewEventRequest {
            chain_id: self.chain_id,
            address: Bytes::from(self.contract_addr.0.to_vec()),
            event_name: "SwapRequested".into(),
            fields: vec![
                omnievent::proto_types::EventField {
                    sol_type: "bytes32".into(),
                    indexed: true,
                },
                omnievent::proto_types::EventField {
                    sol_type: "bytes".into(),
                    indexed: false,
                },
            ],
            block_safety: Default::default(),
        };

        let (_, registered_event, _) = client.register_event(request).await?.into_parts();
        self.events.push(registered_event.uuid);
        Ok(())
    }

    async fn handle(&self, e: &EventOccurrence) -> eyre::Result<()> {
        if !self.events.contains((&e.event_uuid).try_into()?) {
            return Ok(());
        }

        let deposit: BridgeDepositEvent = e.try_into()?;
        let provider = self.multi_provider.get_ethereum_provider(&self.chain_id).ok_or(eyre::eyre!("no provider for chain_id {}", self.chain_id))?;
        let instance = RouterInstance::new(self.contract_addr, provider);
        let transfer = instance.getTransferParameters(deposit.request_id.into()).call().await?;
        println!("sending {} from chain {} to chain {}", transfer.amount, transfer.srcChainId, transfer.dstChainId);

        Ok(())
    }
}

struct BridgeDepositEvent {
    request_id: [u8; 32],
}

impl TryFrom<&EventOccurrence> for BridgeDepositEvent {
    type Error = eyre::Error;

    fn try_from(value: &EventOccurrence) -> Result<Self, Self::Error> {
        if value.event_data.is_empty() {
            Err(eyre!("no event data on bridge deposit event"))?
        }

        let request_id: [u8; 32] = match value.event_data[0].clone().value {
            None => Err(eyre!("no event data on bridge deposit event")),
            Some(event_data::Value::BytesValue(b)) => b.as_ref().try_into().map_err(|e: TryFromSliceError| eyre!("request_id wasn't 32 bytes; err {}", e)),
            Some(_) => Err(eyre!("request_id wasn't bytes")),
        }?;

        Ok(BridgeDepositEvent { request_id })
    }
}
