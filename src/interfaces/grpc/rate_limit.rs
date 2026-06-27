use std::{
    net::SocketAddr,
    task::{Context, Poll},
};

use axum::{
    extract::ConnectInfo,
    http::{Request, Response},
};
use tonic::transport::server::TcpConnectInfo;
use tower::{Layer, Service};

/// `tower_governor`'s `PeerIpKeyExtractor` only looks for an
/// `axum::extract::ConnectInfo<SocketAddr>` extension. Tonic instead exposes
/// the peer address via `TcpConnectInfo`, so this layer copies it into the
/// extension type the rate limiter expects before handing the request off.
#[derive(Clone)]
pub struct ConnectInfoBridge<S> {
    inner: S,
}

impl<S> ConnectInfoBridge<S> {
    fn new(inner: S) -> Self {
        Self { inner }
    }
}

#[derive(Clone, Copy, Default)]
pub struct ConnectInfoBridgeLayer;

impl<S> Layer<S> for ConnectInfoBridgeLayer {
    type Service = ConnectInfoBridge<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ConnectInfoBridge::new(inner)
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for ConnectInfoBridge<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        if let Some(remote_addr) = req
            .extensions()
            .get::<TcpConnectInfo>()
            .and_then(TcpConnectInfo::remote_addr)
        {
            req.extensions_mut()
                .insert(ConnectInfo::<SocketAddr>(remote_addr));
        }
        self.inner.call(req)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use axum::body::Body;
    use tower::{Layer, ServiceExt};
    use tower_governor::GovernorLayer;

    use super::*;
    use crate::interfaces::rate_limit::build_governor_config;

    fn peer_addr() -> SocketAddr {
        "127.0.0.1:54321".parse().unwrap()
    }

    fn request_with_tcp_connect_info() -> Request<Body> {
        let mut req = Request::new(Body::empty());
        req.extensions_mut().insert(TcpConnectInfo {
            local_addr: None,
            remote_addr: Some(peer_addr()),
        });
        req
    }

    #[tokio::test]
    async fn copies_tonic_peer_addr_into_axum_connect_info_extension() {
        let svc = tower::service_fn(|req: Request<Body>| async move {
            let connect_info = req
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|info| info.0);
            Ok::<_, Infallible>(Response::new(connect_info))
        });
        let mut bridged = ConnectInfoBridgeLayer.layer(svc);

        let response = bridged
            .ready()
            .await
            .unwrap()
            .call(request_with_tcp_connect_info())
            .await
            .unwrap();

        assert_eq!(response.into_body(), Some(peer_addr()));
    }

    #[tokio::test]
    async fn governor_layer_rejects_requests_once_burst_is_exhausted() {
        let svc = tower::service_fn(|_req: Request<Body>| async move {
            Ok::<_, Infallible>(Response::new(Body::empty()))
        });
        let governor_cfg = build_governor_config(1, 1).unwrap();
        let mut stack = ConnectInfoBridgeLayer.layer(GovernorLayer::new(governor_cfg).layer(svc));

        let first = stack
            .ready()
            .await
            .unwrap()
            .call(request_with_tcp_connect_info())
            .await
            .unwrap();
        assert_eq!(first.status(), axum::http::StatusCode::OK);

        let second = stack
            .ready()
            .await
            .unwrap()
            .call(request_with_tcp_connect_info())
            .await
            .unwrap();
        assert_eq!(second.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
    }
}
