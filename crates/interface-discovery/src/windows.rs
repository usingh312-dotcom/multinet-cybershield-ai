//! Windows adapter enumeration through `GetAdaptersAddresses` (iphlpapi).
//!
//! The API is read-only. All values come from the OS; nothing is synthesized.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ptr;

use network_core::{
    InterfaceDiscovery, NetworkError, NetworkInterface, OperationalStatus,
};
use windows_sys::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_INCLUDE_PREFIX,
    IP_ADAPTER_ADDRESSES_LH, IP_ADAPTER_GATEWAY_ADDRESS_LH, IP_ADAPTER_UNICAST_ADDRESS_LH,
};
use windows_sys::Win32::NetworkManagement::Ndis::{
    IfOperStatusDormant, IfOperStatusDown, IfOperStatusLowerLayerDown, IfOperStatusNotPresent,
    IfOperStatusTesting, IfOperStatusUnknown, IfOperStatusUp,
};
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6,
};

use crate::{classify_if_type, is_physical_link, looks_virtual};

pub struct WindowsInterfaceDiscovery;

impl InterfaceDiscovery for WindowsInterfaceDiscovery {
    fn discover_interfaces(&self) -> Result<Vec<NetworkInterface>, NetworkError> {
        enumerate()
    }
}

/// Query the OS for every adapter address entry.
pub fn enumerate() -> Result<Vec<NetworkInterface>, NetworkError> {
    let mut size: u32 = 16 * 1024;
    for _ in 0..5 {
        // `Vec<u64>` guarantees alignment for the adapter structures.
        let mut buffer: Vec<u64> = vec![0; (size as usize).div_ceil(8)];
        let result = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_INCLUDE_PREFIX,
                ptr::null(),
                buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                &mut size,
            )
        };

        match result {
            0 => {
                let first = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
                // SAFETY: the OS returned success, so the buffer holds a valid
                // linked list of adapter structures.
                return Ok(unsafe { collect(first) });
            }
            ERROR_BUFFER_OVERFLOW => continue,
            code => {
                return Err(NetworkError::Discovery(format!(
                    "GetAdaptersAddresses failed with error code {code}"
                )));
            }
        }
    }

    Err(NetworkError::Discovery(
        "GetAdaptersAddresses buffer sizing did not converge".to_string(),
    ))
}

unsafe fn collect(mut current: *const IP_ADAPTER_ADDRESSES_LH) -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();
    while !current.is_null() {
        if let Some(interface) = convert(current) {
            interfaces.push(interface);
        }
        current = (*current).Next as *const IP_ADAPTER_ADDRESSES_LH;
    }
    interfaces
}

unsafe fn convert(adapter: *const IP_ADAPTER_ADDRESSES_LH) -> Option<NetworkInterface> {
    let adapter = &*adapter;

    let interface_index = adapter.Anonymous1.Anonymous.IfIndex;
    let name = wide_string(adapter.FriendlyName);
    let description = wide_string(adapter.Description);

    if interface_index == 0 && name.is_empty() && description.is_empty() {
        return None;
    }

    let kind = classify_if_type(adapter.IfType);
    let mac_address = mac_string(&adapter.PhysicalAddress, adapter.PhysicalAddressLength);
    let physical = is_physical_link(
        kind,
        adapter.PhysicalAddressLength as usize,
        looks_virtual(&description, &name),
    );
    let operational_status = map_oper_status(adapter.OperStatus);

    Some(NetworkInterface {
        id: c_string(adapter.AdapterName),
        name,
        description,
        kind,
        if_type: adapter.IfType,
        is_up: operational_status == OperationalStatus::Up,
        operational_status,
        interface_index,
        ipv6_interface_index: adapter.Ipv6IfIndex,
        addresses: unsafe { unicast_addresses(adapter.FirstUnicastAddress) },
        gateways: unsafe { gateway_addresses(adapter.FirstGatewayAddress) },
        mac_address,
        mtu: nonzero_u32(adapter.Mtu),
        transmit_link_speed_bps: nonzero_u64(adapter.TransmitLinkSpeed),
        receive_link_speed_bps: nonzero_u64(adapter.ReceiveLinkSpeed),
        is_physical: physical,
    })
}

unsafe fn unicast_addresses(mut address: *mut IP_ADAPTER_UNICAST_ADDRESS_LH) -> Vec<IpAddr> {
    let mut addresses = Vec::new();
    while !address.is_null() {
        let entry = &*address;
        if let Some(ip) = sockaddr_to_ip(entry.Address.lpSockaddr, entry.Address.iSockaddrLength) {
            addresses.push(ip);
        }
        address = entry.Next;
    }
    addresses
}

unsafe fn gateway_addresses(mut address: *mut IP_ADAPTER_GATEWAY_ADDRESS_LH) -> Vec<IpAddr> {
    let mut gateways = Vec::new();
    while !address.is_null() {
        let entry = &*address;
        if let Some(ip) = sockaddr_to_ip(entry.Address.lpSockaddr, entry.Address.iSockaddrLength) {
            gateways.push(ip);
        }
        address = entry.Next;
    }
    gateways
}

unsafe fn sockaddr_to_ip(sockaddr: *const SOCKADDR, length: i32) -> Option<IpAddr> {
    if sockaddr.is_null() || length <= 0 {
        return None;
    }

    match (*sockaddr).sa_family {
        AF_INET if length as usize >= std::mem::size_of::<SOCKADDR_IN>() => {
            let sin = &*(sockaddr as *const SOCKADDR_IN);
            // `S_addr` is stored in network byte order.
            let raw = sin.sin_addr.S_un.S_addr;
            Some(IpAddr::V4(Ipv4Addr::from(u32::from_be(raw))))
        }
        AF_INET6 if length as usize >= std::mem::size_of::<SOCKADDR_IN6>() => {
            let sin6 = &*(sockaddr as *const SOCKADDR_IN6);
            Some(IpAddr::V6(Ipv6Addr::from(sin6.sin6_addr.u.Byte)))
        }
        _ => None,
    }
}

unsafe fn wide_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

unsafe fn c_string(ptr: *const u8) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf8_lossy(std::slice::from_raw_parts(ptr, len)).into_owned()
}

fn mac_string(bytes: &[u8; 8], length: u32) -> Option<String> {
    let length = (length as usize).min(bytes.len());
    if length < 6 {
        return None;
    }
    Some(
        bytes[..length]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(":"),
    )
}

fn map_oper_status(status: i32) -> OperationalStatus {
    match status {
        IfOperStatusUp => OperationalStatus::Up,
        IfOperStatusDown => OperationalStatus::Down,
        IfOperStatusTesting => OperationalStatus::Testing,
        IfOperStatusUnknown => OperationalStatus::Unknown,
        IfOperStatusDormant => OperationalStatus::Dormant,
        IfOperStatusNotPresent => OperationalStatus::NotPresent,
        IfOperStatusLowerLayerDown => OperationalStatus::LowerLayerDown,
        _ => OperationalStatus::Unknown,
    }
}

fn nonzero_u32(value: u32) -> Option<u32> {
    (value != 0 && value != u32::MAX).then_some(value)
}

fn nonzero_u64(value: u64) -> Option<u64> {
    (value != 0 && value != u64::MAX).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_real_windows_interfaces() {
        let interfaces = WindowsInterfaceDiscovery
            .discover_interfaces()
            .expect("GetAdaptersAddresses should succeed");

        assert!(!interfaces.is_empty(), "expected at least one Windows adapter");

        let mut ids = std::collections::HashSet::new();
        for interface in &interfaces {
            assert!(!interface.id.is_empty(), "adapter id must be populated");
            assert!(!interface.name.is_empty(), "friendly name must be populated");
            assert!(interface.interface_index > 0, "interface index must be populated");
            assert!(ids.insert(interface.id.clone()), "adapter ids must be unique");
            println!("{interface:?}");
        }

        assert!(
            interfaces
                .iter()
                .any(|interface| interface.operational_status == OperationalStatus::Up),
            "expected at least one operational adapter"
        );
    }

    #[test]
    fn discovers_at_least_one_address() {
        let interfaces = WindowsInterfaceDiscovery.discover_interfaces().unwrap();
        let with_addresses = interfaces
            .iter()
            .filter(|interface| !interface.addresses.is_empty())
            .count();
        println!("adapters with addresses: {with_addresses}");
        assert!(with_addresses >= 1, "expected at least one adapter address");
    }

    #[test]
    fn reports_gateways_when_available() {
        let interfaces = WindowsInterfaceDiscovery.discover_interfaces().unwrap();
        for interface in interfaces.iter().filter(|interface| !interface.gateways.is_empty()) {
            println!("gateway for {}: {:?}", interface.name, interface.gateways);
        }
        // Gateways are optional; this test only exercises the walk.
    }
}
