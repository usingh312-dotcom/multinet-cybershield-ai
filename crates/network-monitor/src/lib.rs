//! network-monitor � real per-interface byte counters and throughput rates.
//!
//! Counters come from the Windows IP Helper API (`GetIfTable2`) on Windows. No
//! synthetic values are produced: a rate stays `None` until two real samples
//! exist for an interface, and a backwards counter is reported as a reset.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("interface counters are not available on this platform: {0}")]
    UnsupportedPlatform(String),
    #[error("failed to read interface counters: {0}")]
    Counter(String),
}

/// Raw cumulative counters for one adapter, as reported by the OS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceCounters {
    pub interface_index: u32,
    pub name: String,
    pub description: String,
    /// Raw Windows `IfType` value (6 Ethernet, 71 Wi-Fi, 24 loopback, ...).
    pub if_type: u32,
    pub is_up: bool,
    pub in_octets: u64,
    pub out_octets: u64,
}

/// A sample with a derived rate. `None` means "no honest measurement yet".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceSample {
    pub interface_index: u32,
    pub name: String,
    pub description: String,
    pub if_type: u32,
    pub is_up: bool,
    pub in_octets: u64,
    pub out_octets: u64,
    pub rx_bps: Option<f64>,
    pub tx_bps: Option<f64>,
    /// Bytes accounted since the previous sample (`0` on the first sample and
    /// when a reset is detected).
    pub rx_delta_bytes: u64,
    pub tx_delta_bytes: u64,
    /// The cumulative counter moved backwards (driver reset or reconnect).
    pub counter_reset: bool,
}

impl InterfaceSample {
    pub fn rx_bps_or_zero(&self) -> f64 {
        self.rx_bps.unwrap_or(0.0)
    }

    pub fn tx_bps_or_zero(&self) -> f64 {
        self.tx_bps.unwrap_or(0.0)
    }
}

/// Computes rates from consecutive counter snapshots.
///
/// The caller supplies the elapsed time between snapshots so the math is
/// independent of how often the sampler happens to run.
#[derive(Debug, Default)]
pub struct RateTracker {
    previous: HashMap<u32, (u64, u64)>,
}

impl RateTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a new snapshot. The first snapshot produces `None` rates.
    pub fn update(&mut self, counters: &[InterfaceCounters], elapsed: Duration) -> Vec<InterfaceSample> {
        let seconds = elapsed.as_secs_f64();
        let mut samples = Vec::with_capacity(counters.len());

        for counter in counters {
            let mut counter_reset = false;
            let mut rx_delta_bytes = 0u64;
            let mut tx_delta_bytes = 0u64;
            let (rx_bps, tx_bps) = match self.previous.get(&counter.interface_index) {
                Some(&(prev_in, prev_out)) if seconds > 0.0 => {
                    if counter.in_octets < prev_in || counter.out_octets < prev_out {
                        counter_reset = true;
                        (Some(0.0), Some(0.0))
                    } else {
                        rx_delta_bytes = counter.in_octets - prev_in;
                        tx_delta_bytes = counter.out_octets - prev_out;
                        let rx = rx_delta_bytes as f64 * 8.0 / seconds;
                        let tx = tx_delta_bytes as f64 * 8.0 / seconds;
                        (Some(rx), Some(tx))
                    }
                }
                _ => (None, None),
            };

            self.previous
                .insert(counter.interface_index, (counter.in_octets, counter.out_octets));

            samples.push(InterfaceSample {
                interface_index: counter.interface_index,
                name: counter.name.clone(),
                description: counter.description.clone(),
                if_type: counter.if_type,
                is_up: counter.is_up,
                in_octets: counter.in_octets,
                out_octets: counter.out_octets,
                rx_bps,
                tx_bps,
                rx_delta_bytes,
                tx_delta_bytes,
                counter_reset,
            });
        }

        // Forget adapters that disappeared so a re-added index starts clean.
        let live: Vec<u32> = counters.iter().map(|c| c.interface_index).collect();
        self.previous.retain(|index, _| live.contains(index));

        samples
    }

    pub fn forget(&mut self, interface_index: u32) {
        self.previous.remove(&interface_index);
    }

    pub fn tracked_interfaces(&self) -> usize {
        self.previous.len()
    }
}

/// Read every adapter's cumulative counters from the operating system.
pub fn snapshot_counters() -> Result<Vec<InterfaceCounters>, MonitorError> {
    platform::snapshot_counters()
}

#[cfg(windows)]
mod platform {
    use super::{InterfaceCounters, MonitorError};
    use std::ffi::c_void;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        FreeMibTable, GetIfTable2, MIB_IF_ROW2, MIB_IF_TABLE2,
    };
    use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;

    struct TableGuard(*mut MIB_IF_TABLE2);

    impl Drop for TableGuard {
        fn drop(&mut self) {
            unsafe { FreeMibTable(self.0 as *const c_void) };
        }
    }

    fn wide_to_string(buffer: &[u16]) -> String {
        let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end])
    }

    fn row_to_counters(row: &MIB_IF_ROW2) -> InterfaceCounters {
        InterfaceCounters {
            interface_index: row.InterfaceIndex,
            name: wide_to_string(&row.Alias),
            description: wide_to_string(&row.Description),
            if_type: row.Type,
            is_up: row.OperStatus == IfOperStatusUp,
            in_octets: row.InOctets,
            out_octets: row.OutOctets,
        }
    }

    pub fn snapshot_counters() -> Result<Vec<InterfaceCounters>, MonitorError> {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        let status = unsafe { GetIfTable2(&mut table) };
        if status != 0 || table.is_null() {
            return Err(MonitorError::Counter(format!(
                "GetIfTable2 failed with status {status}"
            )));
        }

        let guard = TableGuard(table);
        let entries = unsafe { (*guard.0).NumEntries as usize };
        if entries == 0 {
            return Ok(Vec::new());
        }

        // `Table` is a one-element array at the end of the struct; the API
        // allocated `NumEntries` contiguous rows behind it.
        let rows = unsafe { std::slice::from_raw_parts((*guard.0).Table.as_ptr(), entries) };
        Ok(rows.iter().map(row_to_counters).collect())
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{InterfaceCounters, MonitorError};

    pub fn snapshot_counters() -> Result<Vec<InterfaceCounters>, MonitorError> {
        Err(MonitorError::UnsupportedPlatform(
            std::env::consts::OS.to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counter(index: u32, in_octets: u64, out_octets: u64) -> InterfaceCounters {
        InterfaceCounters {
            interface_index: index,
            name: format!("if{index}"),
            description: "test adapter".into(),
            if_type: 6,
            is_up: true,
            in_octets,
            out_octets,
        }
    }

    #[test]
    fn first_sample_has_no_rate() {
        let mut tracker = RateTracker::new();
        let samples = tracker.update(&[counter(1, 1_000, 2_000)], Duration::from_secs(1));
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].rx_bps, None);
        assert_eq!(samples[0].tx_bps, None);
        assert!(!samples[0].counter_reset);
    }

    #[test]
    fn rates_use_byte_deltas_over_elapsed_time() {
        let mut tracker = RateTracker::new();
        tracker.update(&[counter(1, 1_000, 2_000)], Duration::from_secs(1));
        // 125_000 bytes in + 250_000 bytes out over 2 s = 500 kbps / 1 Mbps.
        let samples = tracker.update(&[counter(1, 126_000, 252_000)], Duration::from_secs(2));
        assert_eq!(samples[0].rx_bps, Some(500_000.0));
        assert_eq!(samples[0].tx_bps, Some(1_000_000.0));
        assert!(!samples[0].counter_reset);
    }

    #[test]
    fn counter_reset_is_flagged_without_negative_rates() {
        let mut tracker = RateTracker::new();
        tracker.update(&[counter(1, 9_000, 9_000)], Duration::from_secs(1));
        let samples = tracker.update(&[counter(1, 10, 20)], Duration::from_secs(1));
        assert!(samples[0].counter_reset);
        assert_eq!(samples[0].rx_bps, Some(0.0));
        assert_eq!(samples[0].tx_bps, Some(0.0));
    }

    #[test]
    fn disappearing_adapter_is_forgotten() {
        let mut tracker = RateTracker::new();
        tracker.update(&[counter(1, 1, 1), counter(2, 1, 1)], Duration::from_secs(1));
        assert_eq!(tracker.tracked_interfaces(), 2);
        tracker.update(&[counter(2, 2, 2)], Duration::from_secs(1));
        assert_eq!(tracker.tracked_interfaces(), 1);
    }

    #[test]
    fn zero_elapsed_does_not_divide_by_zero() {
        let mut tracker = RateTracker::new();
        tracker.update(&[counter(1, 1, 1)], Duration::from_secs(1));
        let samples = tracker.update(&[counter(1, 100, 100)], Duration::from_secs(0));
        assert_eq!(samples[0].rx_bps, None);
    }

    #[cfg(windows)]
    #[test]
    fn reads_real_hardware_counters() {
        let counters = snapshot_counters().expect("IP Helper counters");
        assert!(!counters.is_empty());
        for c in &counters {
            println!(
                "index={} name={:?} up={} in={} out={}",
                c.interface_index, c.name, c.is_up, c.in_octets, c.out_octets
            );
        }
    }
}
