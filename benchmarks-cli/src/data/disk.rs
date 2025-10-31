use benchmarks_sysinfo::disk::DiskData;
use owo_colors::Style;
use sizef::IntoSize;

use crate::data::{DataProvider, DataRow};

#[derive(Debug, Clone, Default)]
pub struct DiskDataProvider;

impl DataProvider for DiskDataProvider {
    fn identifier(&self) -> &'static str {
        "Disk"
    }
    fn try_fetch(&self) -> Result<Vec<super::DataRow>, String> {
        let data = DiskData::fetch().map_err(|err| err.to_string())?;
        Ok(data
            .disks
            .into_iter()
            .map(|disk| {
                DataRow::new(format!("Disk({})", disk.device_name))
                    .with_value(disk.model, Style::new())
                    .with_value(format!(" [{}]", disk.size.into_decimalsize()), Style::new())
            })
            .collect())
    }
}
