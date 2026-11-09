//! tunnel-protocol — foundation stub (Phase 1). No production logic; no system mutations.
//! Authenticated packet framing lives in [`crate::framing`].

#[allow(dead_code)]
pub fn foundation_stub() -> &'static str {
    "foundation-stub"
}

pub mod framing;
pub use framing::{FrameHeader, FramingError, FramingLayer, PacketFrame};

#[cfg(test)]
mod tests {
    #[test]
    fn stub_exists() {
        assert_eq!(super::foundation_stub(), "foundation-stub");
    }
}
