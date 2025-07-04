use crate::db::InMemoryDatabase;
use alloy::primitives::Address;
use bytes::Bytes;
use omnievent::event_manager::EventManager;
use omnievent::grpc::OmniEventServiceImpl;
use omnievent::proto_types::{RegisterNewEventRequest, StreamEventsRequest};
use omnievent::proto_types::omni_event_service_client::OmniEventServiceClient;
use omnievent::proto_types::omni_event_service_server::OmniEventServiceServer;
use std::str::FromStr;
use std::sync::Arc;
use superalloy::provider::MultiProvider;
use tonic::codegen::tokio_stream::StreamExt;
use crate::config::NetworkConfig;

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

    pub async fn stream(&mut self, networks: &Vec<NetworkConfig>) -> eyre::Result<()> {
        let mut event_ids = Vec::new();
        for network in networks.iter().cloned() {
            let chain_id = network.chain_id;
            let contract_addr = Bytes::from(Address::from_str(&network.router_address.clone())?.0.to_vec());
            let event_specification = RegisterNewEventRequest {
                chain_id,
                address: contract_addr,
                event_name: "RandomnessRequested".into(),
                fields: vec![
                    omnievent::proto_types::EventField {
                        sol_type: "uint256".into(),
                        indexed: true,
                    },
                    omnievent::proto_types::EventField {
                        sol_type: "uint256".into(),
                        indexed: true,
                    },
                    omnievent::proto_types::EventField {
                        sol_type: "address".into(),
                        indexed: true,
                    },
                    omnievent::proto_types::EventField {
                        sol_type: "uint256".into(),
                        indexed: false,
                    },
                ],
                block_safety: Default::default(),
            };
            let response = self.client.register_event(event_specification).await?;
            let (_, registered_event, _) = response.into_parts();
            event_ids.push(registered_event.uuid); 
            
        }

        let (_, mut stream, _) = self.client
            .stream_events(StreamEventsRequest { event_uuids: event_ids })
            .await?
            .into_parts();
        while let Ok(event) = stream.next().await.unwrap() {
            println!("{:?}", event.event_uuid)
        }
        Ok(())
    }
}
