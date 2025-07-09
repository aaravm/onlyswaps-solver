use crate::chain::Chain;
use crate::db::InMemoryDatabase;
use alloy::primitives::{Address, U256};
use bytes::Bytes;
use eyre::eyre;
use itertools::Itertools;
use omnievent::event_manager::EventManager;
use omnievent::grpc::OmniEventServiceImpl;
use omnievent::proto_types::omni_event_service_client::OmniEventServiceClient;
use omnievent::proto_types::omni_event_service_server::OmniEventServiceServer;
use omnievent::proto_types::{EventOccurrence, RegisterNewEventRequest, StreamEventsRequest, event_data};
use std::array::TryFromSliceError;
use std::collections::HashMap;
use std::sync::Arc;
use superalloy::provider::MultiProvider;
use tonic::codegen::tokio_stream::StreamExt;

type OmniEventPlugin = OmniEventServiceServer<OmniEventServiceImpl<Arc<MultiProvider<u64>>, InMemoryDatabase>>;
pub(crate) fn create_omnievent_plugin(provider: Arc<MultiProvider<u64>>) -> eyre::Result<OmniEventPlugin> {
    let db = InMemoryDatabase::default();
    let mut event_manager = EventManager::new(provider, db);
    event_manager.start();
    let omnievent = Arc::new(OmniEventServiceImpl::new(Arc::new(event_manager)));

    Ok(OmniEventServiceServer::from_arc(Arc::clone(&omnievent)))
}

pub(crate) struct PluginHandler {
    client: OmniEventServiceClient<tonic::transport::channel::Channel>,
}

impl PluginHandler {
    pub fn new(port: u16) -> eyre::Result<Self> {
        let url = format!("http://127.0.0.1:{}", port);
        let channel = tonic::transport::Endpoint::new(url)?.connect_lazy();
        let client = OmniEventServiceClient::new(channel);
        Ok(Self { client })
    }

    pub async fn stream(&mut self, networks: &HashMap<U256, Chain>) -> eyre::Result<()> {
        // we need to make sure the event plugin has started first... ugly
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut uuid_to_chain_id = HashMap::new();
        for chain in networks.values() {
            let swap_requested_event = create_swap_requested_event(chain.chain_id, chain.router.address());
            let (_, response, _) = self.client.register_event(swap_requested_event).await?.into_parts();
            uuid_to_chain_id.insert(response.uuid, chain.chain_id);
        }

        let (_, mut stream, _) = self
            .client
            .stream_events(StreamEventsRequest {
                event_uuids: uuid_to_chain_id.keys().cloned().collect_vec(),
            })
            .await?
            .into_parts();

        while let Ok(event) = stream.next().await.ok_or(eyre::eyre!("stream closed"))? {
            let source_chain = uuid_to_chain_id
                .get(&event.event_uuid)
                .map(|chain_id| networks.get(chain_id))
                .flatten()
                .ok_or(eyre::eyre!("no such chain_id"))?;
            let transfer_params = source_chain.fetch_transfer_params((&event).try_into()?).await?;
            match networks.get(&transfer_params.dest_chain_id) {
                None => println!("skipping transfer for chain_id {} as we don't support it", transfer_params.dest_chain_id),
                Some(dest_chain) => dest_chain.attempt_token_relay_if_profitable(transfer_params).await?,
            }
        }

        Err(eyre!("event stream stopped unexpectedly!"))
    }
}

pub(crate) struct BridgeDepositEvent {
    pub request_id: [u8; 32],
}

impl TryFrom<&EventOccurrence> for BridgeDepositEvent {
    type Error = eyre::Error;

    fn try_from(value: &EventOccurrence) -> Result<Self, Self::Error> {
        if value.event_data.is_empty() {
            Err(eyre!("no event data on bridge deposit event"))?
        }

        let request_id: [u8; 32] = match value.event_data[0].clone().value {
            Some(event_data::Value::BytesValue(b)) => b
                .as_ref()
                .try_into()
                .map_err(|e: TryFromSliceError| eyre!("request_id wasn't 32 bytes; err {}", e)),
            Some(_) => Err(eyre!("request_id wasn't bytes")),
            None => Err(eyre!("no event data on bridge deposit event")),
        }?;

        Ok(BridgeDepositEvent { request_id })
    }
}

fn create_swap_requested_event(chain_id: U256, router_address: &Address) -> RegisterNewEventRequest {
    let probably_always_valid_chain_id = chain_id.as_limbs()[0];
    RegisterNewEventRequest {
        chain_id: probably_always_valid_chain_id,
        address: Bytes::from(router_address.0.to_vec()),
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
    }
}
