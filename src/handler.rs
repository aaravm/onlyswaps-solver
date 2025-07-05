use alloy::primitives::Address;
use bytes::Bytes;
use omnievent::proto_types::{EventOccurrence, RegisterNewEventRequest};
use omnievent::proto_types::omni_event_service_client::OmniEventServiceClient;

pub(crate) trait EventHandler {
    fn register(&mut self, client: &mut OmniEventServiceClient<tonic::transport::channel::Channel>) -> impl Future<Output = eyre::Result<()>>;
    fn handle(&self, e: &EventOccurrence) -> impl Future<Output = eyre::Result<()>>;
}

pub(crate) struct BridgeDepositHandler {
    chain_id: u64,
    contract_addr: Address,
    pub events: Vec<Bytes>,
}

impl BridgeDepositHandler {
    pub fn new(chain_id: u64, contract_addr: Address) -> Self {
        Self {
            chain_id, contract_addr, events: Vec::default(),
        }
    }
}

 impl EventHandler for BridgeDepositHandler {
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

        println!("handling event {:?}", e.event_uuid);
        Ok(())
    }
}
