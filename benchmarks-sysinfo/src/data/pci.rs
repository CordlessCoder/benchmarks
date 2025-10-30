use rxfetch::pci::PciDevIterBackend;
pub use rxfetch::pci::{AutoProvider, PciBackendError, PciDevice};
use tracing::warn;

use crate::util::{Device, query_pci_devices};

pub struct PCIData {
    pub all_devices: Vec<PciDevice<AutoProvider>>,
    pub all_devices_named: Vec<Device>,
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
        // let gpus = gpu_queries
        //     .into_iter()
        //     .flat_map(|(vendor_id, device_id)| {
        //         all_devices_named
        //             .iter()
        //             .position(|dev| dev.vid == vendor_id && dev.did == device_id)
        //     })
        //     .collect();

        Ok(PCIData {
            all_devices,
            all_devices_named,
        })
    }
    pub fn gpus(&self) -> impl Iterator<Item = &Device> {
        self.all_devices_named.iter().filter(|dev| dev.is_gpu)
    }
}
