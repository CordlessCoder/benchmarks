use crate::{
    data::{DataProvider, DataRow, pci_totals::PCI_DEVICE_CACHE},
};
use benchmarks_sysinfo::util::PrettyDevice;
use owo_colors::Style;

#[derive(Debug)]
pub struct GpuDataProvider;

impl DataProvider for GpuDataProvider {
    fn identifier(&self) -> &'static str {
        "GPU"
    }
    fn try_fetch(&self) -> Result<Vec<DataRow>, String> {
        let Some(data) = &*PCI_DEVICE_CACHE else {
            return Err("Fetching PCI data failed".to_string());
        };
        let mut rows: Vec<DataRow> = data
            .lock()
            .unwrap()
            .gpus
            .iter()
            .map(|gpu| DataRow::new("GPU").with_value(PrettyDevice(gpu).to_string(), Style::new()))
            .collect();
        if rows.len() > 1 {
            rows.iter_mut().enumerate().for_each(|(i, row)| {
                use core::fmt::Write;
                _ = write!(row.label.to_mut(), " {}", i + 1);
            });
        }
        Ok(rows)
    }
}
