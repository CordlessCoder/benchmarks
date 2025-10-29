use crate::data::{DataProvider, DataRow};
use benchmarks_sysinfo::pci::PCIData;
use owo_colors::Style;
use std::sync::{LazyLock, Mutex};
use tracing::warn;

pub static PCI_DEVICE_CACHE: LazyLock<Option<Mutex<PCIData>>> = LazyLock::new(|| {
    let data = PCIData::fetch()
        .inspect_err(|_err| warn!("PCI Error emitted by backend: {_err:?}"))
        .ok()?;
    Some(Mutex::new(data))
});

#[derive(Debug)]
pub struct PciTotalProvider;
impl DataProvider for PciTotalProvider {
    fn identifier(&self) -> &'static str {
        "PCI"
    }
    fn try_fetch(&self) -> Result<Vec<super::DataRow>, String> {
        let Some(data) = &*PCI_DEVICE_CACHE else {
            return Err("Fetching PCI data failed".to_string());
        };
        let count = data.lock().unwrap().all_devices.len();
        Ok(vec![
            DataRow::new("PCI Devices").with_value(count.to_string(), Style::new()),
        ])
    }
}
