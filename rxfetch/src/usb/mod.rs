// #[cfg(feature = "tracing")]
// use tracing::instrument;
#[cfg(unix)]
mod linux;

#[cfg(unix)]
pub use linux::*;

#[derive(Debug, Clone)]
pub struct UsbDeviceID {
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
}
