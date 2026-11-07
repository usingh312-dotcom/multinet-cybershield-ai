use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("platform unsupported: {0}")]
    UnsupportedPlatform(String),
    #[error("discovery failed: {0}")]
    Discovery(String),
    #[error("route configuration rejected: {0}")]
    RouteConfig(String),
    #[error("not implemented in foundation phase: {0}")]
    NotImplemented(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkInterface {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub is_up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InterfaceMetrics {
    pub interface_id: String,
    pub latency_ms: Option<f64>,
    pub loss_pct: Option<f64>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteInfo {
    pub destination: String,
    pub gateway: Option<String>,
    pub interface_id: Option<String>,
    pub metric: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VirtualInterfaceSpec {
    pub name: String,
    pub mtu: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteChange {
    pub description: String,
}

#[async_trait]
pub trait NetworkPlatform: Send + Sync {
    async fn discover_interfaces(&self) -> Result<Vec<NetworkInterface>, NetworkError>;
    async fn get_interface_metrics(
        &self,
        interface_id: &str,
    ) -> Result<InterfaceMetrics, NetworkError>;
    async fn get_default_route(&self) -> Result<Option<RouteInfo>, NetworkError>;
    async fn bind_transport(&self, interface_id: &str) -> Result<(), NetworkError>;
    async fn create_virtual_interface(
        &self,
        spec: VirtualInterfaceSpec,
    ) -> Result<NetworkInterface, NetworkError>;
    async fn configure_routes(&self, changes: Vec<RouteChange>) -> Result<(), NetworkError>;
    async fn restore_network(&self) -> Result<(), NetworkError>;
}

pub struct MockNetworkPlatform;

#[async_trait]
impl NetworkPlatform for MockNetworkPlatform {
    async fn discover_interfaces(&self) -> Result<Vec<NetworkInterface>, NetworkError> {
        Ok(vec![])
    }

    async fn get_interface_metrics(
        &self,
        interface_id: &str,
    ) -> Result<InterfaceMetrics, NetworkError> {
        Ok(InterfaceMetrics {
            interface_id: interface_id.to_string(),
            latency_ms: None,
            loss_pct: None,
            rx_bytes: 0,
            tx_bytes: 0,
        })
    }

    async fn get_default_route(&self) -> Result<Option<RouteInfo>, NetworkError> {
        Ok(None)
    }

    async fn bind_transport(&self, _interface_id: &str) -> Result<(), NetworkError> {
        Err(NetworkError::NotImplemented(
            "bind_transport is a foundation stub".to_string(),
        ))
    }

    async fn create_virtual_interface(
        &self,
        spec: VirtualInterfaceSpec,
    ) -> Result<NetworkInterface, NetworkError> {
        Ok(NetworkInterface {
            id: Uuid::new_v4().to_string(),
            name: spec.name,
            kind: "virtual-mock".to_string(),
            is_up: false,
        })
    }

    async fn configure_routes(&self, _changes: Vec<RouteChange>) -> Result<(), NetworkError> {
        Err(NetworkError::NotImplemented(
            "route changes are disabled in foundation phase".to_string(),
        ))
    }

    async fn restore_network(&self) -> Result<(), NetworkError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_platform_is_safe() {
        let p = MockNetworkPlatform;
        assert!(p.discover_interfaces().await.unwrap().is_empty());
        assert!(p.get_default_route().await.unwrap().is_none());
        assert!(p.restore_network().await.is_ok());
    }
}
