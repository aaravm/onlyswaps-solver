use crate::db::InMemoryDatabase;
use alloy::primitives::Address;
use omnievent::event_manager::EventManager;
use omnievent::grpc::OmniEventServiceImpl;
use omnievent::proto_types::StreamEventsRequest;
use omnievent::proto_types::omni_event_service_client::OmniEventServiceClient;
use omnievent::proto_types::omni_event_service_server::OmniEventServiceServer;
use std::str::FromStr;
use std::sync::Arc;
use eyre::eyre;
use itertools::Itertools;
use superalloy::provider::MultiProvider;
use tonic::codegen::tokio_stream::StreamExt;
use crate::config::NetworkConfig;
use crate::handler::{BridgeDepositHandler, EventHandler};

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
        let mut handlers = Vec::new();
        for network in networks.iter().cloned() {
            let chain_id = network.chain_id;
            let router_addr = Address::from_str(&network.router_address.clone())?;
            let mut handler = BridgeDepositHandler::new(chain_id, router_addr);
            handler.register(&mut self.client).await?;
            handlers.push(handler);
        }
        let event_ids = handlers.iter()
            .map(|it| it.events.clone())
            .flatten()
            .collect_vec();

        let (_, mut stream, _) = self.client
            .stream_events(StreamEventsRequest { event_uuids: event_ids })
            .await?
            .into_parts();
        
        while let Ok(event) = stream.next().await.unwrap() {
            for handler in &handlers {
                handler.handle(&event).await?
            }
        }
        
        Err(eyre!("event stream stopped unexpectedly!"))
    }
}
