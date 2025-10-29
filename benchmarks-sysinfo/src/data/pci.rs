use crate::util::{Device, read_pci_device};
use rxfetch::pci::PciDevIterBackend;
pub use rxfetch::pci::{AutoProvider, PciBackendError, PciDevice};
use tracing::warn;

pub struct PCIData {
    pub all_devices: Vec<PciDevice<AutoProvider>>,
    pub gpus: Vec<Device>,
}

impl PCIData {
    pub fn fetch() -> Result<Self, PciBackendError> {
        let device_iter = rxfetch::pci::PciAutoIter::try_init()?;
        let mut all_devices: Vec<PciDevice<_>> = device_iter
            .filter_map(|dev| {
                dev.inspect_err(|_err| warn!("Failed to enumerate PCI device {_err}"))
                    .ok()
            })
            .collect();
        let gpus = all_devices
            .iter_mut()
            .filter_map(|dev| dev.is_gpu().unwrap_or(false).then_some(dev))
            .filter_map(|dev| {
                read_pci_device(
                    dev.vendor()
                        .inspect_err(|_err| warn!("Failed to get PCI device vendor id {_err}"))
                        .ok()?,
                    dev.device()
                        .inspect_err(|_err| warn!("Failed to get PCI device id: {_err}"))
                        .ok()?,
                )
                .inspect_err(|_err| warn!("Failed to find device in pciid database: {_err}"))
                .ok()
                .flatten()
            })
            .collect();
        Ok(PCIData { gpus, all_devices })
    }
}
