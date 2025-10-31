use crate::util::{pretty_pci_device::NamedPciDevice, query_pcidb::query_pci_devices};
use rxfetch::pci::PciDevIterBackend;
pub use rxfetch::pci::{AutoProvider, PciBackendError, PciDevice};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct PCIData {
    pub all_devices: Vec<PciDevice<AutoProvider>>,
    pub all_devices_named: Vec<NamedPciDevice>,
}

impl PCIData {
    pub fn fetch() -> Result<Self, PciBackendError> {
        let device_iter = rxfetch::pci::PciAutoIter::try_init()?;
        let mut all_devices: Vec<PciDevice<_>> = device_iter
            .filter_map(|dev| {
                dev.inspect_err(|err| warn!("Failed to enumerate PCI device {err}"))
                    .ok()
            })
            .collect();
        let queries = all_devices.iter_mut().flat_map(|dev| {
            Some((
                dev.vendor()
                    .inspect_err(|err| warn!("Failed to get PCI device vendor id {err}"))
                    .ok()?,
                dev.device()
                    .inspect_err(|err| warn!("Failed to get PCI device id: {err}"))
                    .ok()?,
                dev.is_gpu().unwrap_or(false),
            ))
        });
        let all_devices_named =
            query_pci_devices(queries).map_err(rxfetch::pci::PciBackendError::IOError)?;

        Ok(PCIData {
            all_devices,
            all_devices_named,
        })
    }
    pub fn gpus(&self) -> impl Iterator<Item = &NamedPciDevice> {
        self.all_devices_named.iter().filter(|dev| dev.is_gpu)
    }
}
