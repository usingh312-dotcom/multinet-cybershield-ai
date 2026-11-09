use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
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

/// Coarse classification derived from the platform interface type and adapter
/// description. It is intentionally lossy; `if_type` preserves the raw value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceKind {
    Ethernet,
    Wifi,
    Loopback,
    Tunnel,
    Ppp,
    Virtual,
    Other,
}

/// Normalized operational status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationalStatus {
    Up,
    Down,
    Dormant,
    Testing,
    Unknown,
    NotPresent,
    LowerLayerDown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkInterface {
    /// Stable platform identifier (Windows: adapter name / GUID).
    pub id: String,
    /// Human friendly name.
    pub name: String,
    /// Driver/adapter description.
    pub description: String,
    pub kind: InterfaceKind,
    /// Raw platform interface type (Windows `IfType`).
    pub if_type: u32,
    pub is_up: bool,
    pub operational_status: OperationalStatus,
    /// IPv4 interface index (Windows `IfIndex`).
    pub interface_index: u32,
    /// IPv6 interface index (Windows `Ipv6IfIndex`).
    pub ipv6_interface_index: u32,
    pub addresses: Vec<IpAddr>,
    pub gateways: Vec<IpAddr>,
    /// Colon-separated MAC when the adapter reports one.
    pub mac_address: Option<String>,
    pub mtu: Option<u32>,
    pub transmit_link_speed_bps: Option<u64>,
    pub receive_link_speed_bps: Option<u64>,
    /// True when the adapter looks like a physical (non-virtual, non-tunnel)
    /// link suitable for path bonding.
    pub is_physical: bool,
}

impl NetworkInterface {
    pub fn has_ipv4(&self) -> bool {
        self.addresses.iter().any(IpAddr::is_ipv4)
    }

    pub fn has_ipv6(&self) -> bool {
        self.addresses.iter().any(IpAddr::is_ipv6)
    }

    pub fn first_address_of_family(&self, ipv4: bool) -> Option<IpAddr> {
        self.addresses
            .iter()
            .copied()
            .find(|addr| addr.is_ipv4() == ipv4)
    }

    /// A path is eligible when the adapter is a live physical link with at
    /// least one usable address. Loopback, tunnel and virtual adapters are
    /// excluded regardless of their operational state.
    pub fn is_eligible_for_bonding(&self) -> bool {
        self.is_physical
            && self.is_up
            && self.kind != InterfaceKind::Loopback
            && self.kind != InterfaceKind::Tunnel
            && self.kind != InterfaceKind::Ppp
            && (self.has_ipv4() || self.has_ipv6())
    }
}

/// Synchronous interface enumeration. The implementation is expected to query
/// the platform directly; values are never fabricated.
pub trait InterfaceDiscovery: Send + Sync {
    fn discover_interfaces(&self) -> Result<Vec<NetworkInterface>, NetworkError>;
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
            description: String::new(),
            kind: InterfaceKind::Virtual,
            if_type: 0,
            is_up: false,
            operational_status: OperationalStatus::Unknown,
            interface_index: 0,
            ipv6_interface_index: 0,
            addresses: Vec::new(),
            gateways: Vec::new(),
            mac_address: None,
            mtu: Some(spec.mtu),
            transmit_link_speed_bps: None,
            receive_link_speed_bps: None,
            is_physical: false,
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

    fn interface(kind: InterfaceKind, is_physical: bool, is_up: bool, addresses: Vec<IpAddr>) -> NetworkInterface {
        NetworkInterface {
            id: "test".to_string(),
            name: "test".to_string(),
            description: String::new(),
            kind,
            if_type: 0,
            is_up,
            operational_status: if is_up {
                OperationalStatus::Up
            } else {
                OperationalStatus::Down
            },
            interface_index: 1,
            ipv6_interface_index: 1,
            addresses,
            gateways: Vec::new(),
            mac_address: None,
            mtu: None,
            transmit_link_speed_bps: None,
            receive_link_speed_bps: None,
            is_physical,
        }
    }

    #[test]
    fn eligibility_requires_physical_up_adapter_with_address() {
        let v4: IpAddr = "192.0.2.10".parse().unwrap();
        assert!(interface(InterfaceKind::Ethernet, true, true, vec![v4]).is_eligible_for_bonding());
        assert!(!interface(InterfaceKind::Ethernet, true, false, vec![v4]).is_eligible_for_bonding());
        assert!(!interface(InterfaceKind::Ethernet, false, true, vec![v4]).is_eligible_for_bonding());
        assert!(!interface(InterfaceKind::Loopback, true, true, vec![v4]).is_eligible_for_bonding());
        assert!(!interface(InterfaceKind::Tunnel, true, true, vec![v4]).is_eligible_for_bonding());
        assert!(!interface(InterfaceKind::Ethernet, true, true, vec![]).is_eligible_for_bonding());
    }

    #[test]
    fn address_family_helpers_are_exact() {
        let iface = interface(
            InterfaceKind::Ethernet,
            true,
            true,
            vec!["192.0.2.10".parse().unwrap(), "2001:db8::1".parse().unwrap()],
        );
        assert!(iface.has_ipv4());
        assert!(iface.has_ipv6());
        assert_eq!(iface.first_address_of_family(true), Some("192.0.2.10".parse().unwrap()));
        assert_eq!(iface.first_address_of_family(false), Some("2001:db8::1".parse().unwrap()));
    }
}
