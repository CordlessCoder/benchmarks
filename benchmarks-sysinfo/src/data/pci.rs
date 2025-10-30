use rxfetch::pci::PciDevIterBackend;
pub use rxfetch::pci::{AutoProvider, PciBackendError, PciDevice};
use tracing::warn;

use crate::util::{Device, query_pci_devices};

pub struct PCIData {
    pub all_devices: Vec<PciDevice<AutoProvider>>,
    pub all_devices_named: Vec<Device>,
    // These indices of all_devices_named are GPUs
    pub gpus: Vec<usize>,
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
        let mut gpu_queries = Vec::new();
        let queries = all_devices.iter_mut().flat_map(|dev| {
            let query = (
                dev.vendor()
                    .inspect_err(|_err| warn!("Failed to get PCI device vendor id {_err}"))
                    .ok()?,
                dev.device()
                    .inspect_err(|_err| warn!("Failed to get PCI device id: {_err}"))
                    .ok()?,
            );
            if dev.is_gpu().unwrap_or(false) {
                gpu_queries.push(query);
            }
            Some(query)
        });
        let all_devices_named =
            query_pci_devices(queries).map_err(rxfetch::pci::PciBackendError::IOError)?;
        let gpus = gpu_queries
            .into_iter()
            .flat_map(|(vendor_id, device_id)| {
                all_devices_named
                    .iter()
                    .position(|dev| dev.vid == vendor_id && dev.did == device_id)
            })
            .collect();

        Ok(PCIData {
            gpus,
            all_devices,
            all_devices_named,
        })
    }
    pub fn gpus(&self) -> impl Iterator<Item = &Device> {
        self.gpus
            .iter()
            .flat_map(|&idx| self.all_devices_named.get(idx))
    }
}
