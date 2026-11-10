use network_core::NetworkInterface;
use std::fmt;
use std::net::IpAddr;

#[derive(Debug, PartialEq, Eq)]
pub enum InterfaceValidationError {
    MalformedIp(String),
    InterfaceNotFound(String),
    IpInterfaceMismatch { ip: IpAddr, interface: String },
    UnassignedIp(IpAddr),
    CannotBind { ip: IpAddr, error: String },
}

impl fmt::Display for InterfaceValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedIp(msg) => write!(f, "{}", msg),
            Self::InterfaceNotFound(name) => {
                write!(f, "Network interface '{}' not found on system", name)
            }
            Self::IpInterfaceMismatch { ip, interface } => write!(
                f,
                "IP {} is not assigned to specified interface '{}'",
                ip, interface
            ),
            Self::UnassignedIp(ip) => write!(
                f,
                "Local IP {} is not assigned to any discovered network interface",
                ip
            ),
            Self::CannotBind { ip, error } => {
                write!(f, "Cannot bind local socket to IP {}: {}", ip, error)
            }
        }
    }
}

impl std::error::Error for InterfaceValidationError {}

/// Validates interface IP and optional interface selection.
///
/// Fails closed if an explicit IP or interface is malformed, unassigned, mismatched, or cannot be bound.
/// Returns `Ok(None)` only if the IP argument was genuinely omitted.
pub fn validate_interface_and_ip(
    raw_ip: Option<&str>,
    interface_name: Option<&str>,
    discovered_interfaces: &[NetworkInterface],
    check_bind: bool,
) -> Result<Option<IpAddr>, InterfaceValidationError> {
    let raw = match raw_ip {
        Some(s) if !s.trim().is_empty() => s.trim(),
        _ => {
            // Genuinely omitted IP retains default behavior
            return Ok(None);
        }
    };

    // 1. Explicit parsing - never silently ignore or fall back to default route on malformed input
    let ip = raw.parse::<IpAddr>().map_err(|e| {
        InterfaceValidationError::MalformedIp(format!(
            "Invalid IP address '{}': must be a valid IPv4 or IPv6 address ({})",
            raw, e
        ))
    })?;

    // 2. If an interface was explicitly specified, verify it exists and owns this IP
    if let Some(iface_name) = interface_name {
        let matching_iface = discovered_interfaces.iter().find(|i| {
            i.name.eq_ignore_ascii_case(iface_name) || i.id.eq_ignore_ascii_case(iface_name)
        });

        match matching_iface {
            None => {
                return Err(InterfaceValidationError::InterfaceNotFound(
                    iface_name.to_string(),
                ));
            }
            Some(iface) => {
                if !iface.addresses.contains(&ip) {
                    return Err(InterfaceValidationError::IpInterfaceMismatch {
                        ip,
                        interface: iface_name.to_string(),
                    });
                }
            }
        }
    } else if !discovered_interfaces.is_empty() {
        // Interface not specified, but interfaces were discovered.
        // Verify IP is assigned to at least one adapter, or is loopback.
        let is_assigned = discovered_interfaces
            .iter()
            .any(|iface| iface.addresses.contains(&ip))
            || ip.is_loopback();

        if !is_assigned {
            return Err(InterfaceValidationError::UnassignedIp(ip));
        }
    }

    // 3. Verify socket binding if requested
    if check_bind {
        if let Err(e) = std::net::UdpSocket::bind((ip, 0)) {
            return Err(InterfaceValidationError::CannotBind {
                ip,
                error: e.to_string(),
            });
        }
    }

    Ok(Some(ip))
}

#[cfg(test)]
mod tests {
    use super::*;
    use network_core::{InterfaceKind, OperationalStatus};

    fn make_mock_interface(name: &str, ips: Vec<IpAddr>) -> NetworkInterface {
        NetworkInterface {
            id: format!("mock-{}", name),
            name: name.to_string(),
            description: format!("Mock adapter {}", name),
            kind: InterfaceKind::Ethernet,
            if_type: 6,
            is_up: true,
            operational_status: OperationalStatus::Up,
            interface_index: 1,
            ipv6_interface_index: 1,
            addresses: ips,
            gateways: vec![],
            mac_address: Some("00:11:22:33:44:55".to_string()),
            mtu: Some(1500),
            transmit_link_speed_bps: Some(1_000_000_000),
            receive_link_speed_bps: Some(1_000_000_000),
            is_physical: true,
        }
    }

    #[test]
    fn test_valid_ipv4_address() {
        let ip_str = "192.168.1.50";
        let parsed_ip: IpAddr = ip_str.parse().unwrap();
        let ifaces = vec![make_mock_interface("eth0", vec![parsed_ip])];

        let result = validate_interface_and_ip(Some(ip_str), None, &ifaces, false);
        assert_eq!(result, Ok(Some(parsed_ip)));
    }

    #[test]
    fn test_valid_ipv6_address() {
        let ip_str = "2001:db8::1";
        let parsed_ip: IpAddr = ip_str.parse().unwrap();
        let ifaces = vec![make_mock_interface("eth0", vec![parsed_ip])];

        let result = validate_interface_and_ip(Some(ip_str), None, &ifaces, false);
        assert_eq!(result, Ok(Some(parsed_ip)));
    }

    #[test]
    fn test_malformed_ipv4_address() {
        let ifaces = vec![make_mock_interface(
            "eth0",
            vec!["192.168.1.1".parse().unwrap()],
        )];

        let result = validate_interface_and_ip(Some("192.168.1.999"), None, &ifaces, false);
        assert!(matches!(
            result,
            Err(InterfaceValidationError::MalformedIp(_))
        ));

        let result_trunc = validate_interface_and_ip(Some("192.168.1"), None, &ifaces, false);
        assert!(matches!(
            result_trunc,
            Err(InterfaceValidationError::MalformedIp(_))
        ));
    }

    #[test]
    fn test_malformed_ipv6_address() {
        let ifaces = vec![];
        let result = validate_interface_and_ip(Some("2001:xyz::1"), None, &ifaces, false);
        assert!(matches!(
            result,
            Err(InterfaceValidationError::MalformedIp(_))
        ));
    }

    #[test]
    fn test_unassigned_local_ip() {
        let assigned: IpAddr = "192.168.1.10".parse().unwrap();
        let unassigned: IpAddr = "10.254.254.254".parse().unwrap();
        let ifaces = vec![make_mock_interface("eth0", vec![assigned])];

        let result = validate_interface_and_ip(Some("10.254.254.254"), None, &ifaces, false);
        assert_eq!(
            result,
            Err(InterfaceValidationError::UnassignedIp(unassigned))
        );
    }

    #[test]
    fn test_ip_interface_mismatch() {
        let ip_eth0: IpAddr = "192.168.1.10".parse().unwrap();
        let ip_wlan0: IpAddr = "192.168.2.20".parse().unwrap();
        let ifaces = vec![
            make_mock_interface("eth0", vec![ip_eth0]),
            make_mock_interface("wlan0", vec![ip_wlan0]),
        ];

        // Supply eth0's IP with interface wlan0
        let result = validate_interface_and_ip(Some("192.168.1.10"), Some("wlan0"), &ifaces, false);
        assert_eq!(
            result,
            Err(InterfaceValidationError::IpInterfaceMismatch {
                ip: ip_eth0,
                interface: "wlan0".to_string()
            })
        );
    }

    #[test]
    fn test_explicit_malformed_ip_never_falls_back_to_default_route() {
        let ifaces = vec![make_mock_interface(
            "eth0",
            vec!["127.0.0.1".parse().unwrap()],
        )];

        // Ensure that passing bad input returns Err and never Ok(None)
        let result = validate_interface_and_ip(Some("invalid-ip-string"), None, &ifaces, false);
        assert!(result.is_err());
        assert_ne!(result, Ok(None));
    }

    #[test]
    fn test_omitted_optional_ip_retains_intended_behavior() {
        let ifaces = vec![make_mock_interface(
            "eth0",
            vec!["192.168.1.1".parse().unwrap()],
        )];

        let result = validate_interface_and_ip(None, None, &ifaces, false);
        assert_eq!(result, Ok(None));

        let result_empty = validate_interface_and_ip(Some(""), None, &ifaces, false);
        assert_eq!(result_empty, Ok(None));
    }
}
