use axum::http::Request;
use std::convert::Infallible;
use tonic::body::Body;
use tonic::codegen::Service;
use tonic::server::NamedService;
use tonic::transport::server::Router;

pub(crate) struct PluginServer {
    router: Option<Router>,
    port: u16,
}

impl PluginServer {
    pub fn new<S>(plugins: Vec<S>, port: u16) -> Self
    where
        S: Service<Request<Body>, Error = Infallible> + NamedService + Clone + Send + Sync + 'static,
        S::Response: axum::response::IntoResponse,
        S::Future: Send + 'static,
    {
        let mut server = tonic::transport::Server::builder();
        let mut router: Option<Router> = None;
        for plugin in plugins {
             router = Some(server.add_service(plugin));
        }
        Self { router, port }
    }

    pub async fn start(self) -> eyre::Result<()> {
        let url = format!("0.0.0.0:{}", self.port).parse()?;
        match self.router {
            None => Err(eyre::eyre!("there were no plugins to start")),
            Some(r) => r.serve(url).await.map_err(Into::into),
        }
    }
}
