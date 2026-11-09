//! interface-discovery — real Windows network adapter enumeration via the
//! IP Helper API (`GetAdaptersAddresses`). No system mutations.

use network_core::{InterfaceKind, InterfaceDiscovery, NetworkError, NetworkInterface};

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::WindowsInterfaceDiscovery;

/// Fallback used on non-Windows targets. Real discovery is Windows-only in
/// this stage; it never fabricates interfaces.
#[cfg(not(windows))]
pub struct WindowsInterfaceDiscovery;

#[cfg(not(windows))]
impl InterfaceDiscovery for WindowsInterfaceDiscovery {
    fn discover_interfaces(&self) -> Result<Vec<NetworkInterface>, NetworkError> {
        Err(NetworkError::UnsupportedPlatform(
            "interface discovery is implemented for Windows only".to_string(),
        ))
    }
}

/// Convenience wrapper around the platform discovery implementation.
pub fn discover_interfaces() -> Result<Vec<NetworkInterface>, NetworkError> {
    WindowsInterfaceDiscovery.discover_interfaces()
}

/// Map a raw Windows `IfType` value to a coarse classification.
pub fn classify_if_type(if_type: u32) -> InterfaceKind {
    match if_type {
        6 => InterfaceKind::Ethernet,
        71 => InterfaceKind::Wifi,
        24 => InterfaceKind::Loopback,
        131 => InterfaceKind::Tunnel,
        23 | 108 => InterfaceKind::Ppp,
        _ => InterfaceKind::Other,
    }
}

/// Adapter descriptions that indicate a software/virtual/tunnel link even when
/// the OS reports an Ethernet-like `IfType`.
const VIRTUAL_KEYWORDS: &[&str] = &[
    "virtual",
    "hyper-v",
    "vethernet",
    "vmware",
    "virtualbox",
    "vbox",
    "npcap",
    "winpcap",
    "loopback",
    "tap-",
    "tap adapter",
    "openvpn",
    "wireguard",
    "tunnel",
    "teredo",
    "isatap",
    "6to4",
    "pseudo",
    "wan miniport",
    "wi-fi direct",
    "wireless direct",
    "kernel debug",
    "zerotier",
    "tailscale",
    "hamachi",
    "docker",
    "wsl",
    "container",
    "vpn",
];

/// True when the adapter name/description identifies a virtual or software
/// link. Heuristic by design; the raw `IfType` is still preserved.
pub fn looks_virtual(description: &str, name: &str) -> bool {
    let haystack = format!("{} {}", description, name).to_ascii_lowercase();
    VIRTUAL_KEYWORDS.iter().any(|keyword| haystack.contains(keyword))
}

/// A link is treated as physical when it is Ethernet/Wi-Fi, is not flagged as
/// virtual, and reports a hardware MAC address.
pub fn is_physical_link(kind: InterfaceKind, mac_length: usize, looks_virtual: bool) -> bool {
    matches!(kind, InterfaceKind::Ethernet | InterfaceKind::Wifi) && !looks_virtual && mac_length >= 6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_interface_types() {
        assert_eq!(classify_if_type(6), InterfaceKind::Ethernet);
        assert_eq!(classify_if_type(71), InterfaceKind::Wifi);
        assert_eq!(classify_if_type(24), InterfaceKind::Loopback);
        assert_eq!(classify_if_type(131), InterfaceKind::Tunnel);
        assert_eq!(classify_if_type(23), InterfaceKind::Ppp);
        assert_eq!(classify_if_type(108), InterfaceKind::Ppp);
        assert_eq!(classify_if_type(0), InterfaceKind::Other);
    }

    #[test]
    fn detects_virtual_adapters() {
        assert!(looks_virtual("Hyper-V Virtual Ethernet Adapter", "vEthernet (Default Switch)"));
        assert!(looks_virtual("", "VMware Network Adapter VMnet8"));
        assert!(looks_virtual("TAP-Windows Adapter V9", ""));
        assert!(looks_virtual("Microsoft Wi-Fi Direct Virtual Adapter", ""));
        assert!(!looks_virtual("Intel(R) Ethernet Connection I219-V", "Ethernet"));
        assert!(!looks_virtual("Intel(R) Wi-Fi 6 AX201 160MHz", "Wi-Fi"));
    }

    #[test]
    fn physical_requires_mac_and_non_virtual() {
        assert!(is_physical_link(InterfaceKind::Ethernet, 6, false));
        assert!(is_physical_link(InterfaceKind::Wifi, 6, false));
        assert!(!is_physical_link(InterfaceKind::Ethernet, 6, true));
        assert!(!is_physical_link(InterfaceKind::Ethernet, 0, false));
        assert!(!is_physical_link(InterfaceKind::Loopback, 6, false));
        assert!(!is_physical_link(InterfaceKind::Tunnel, 6, false));
    }

    #[test]
    fn non_windows_fallback_never_fabricates() {
        #[cfg(not(windows))]
        {
            assert!(discover_interfaces().is_err());
        }
    }
}
