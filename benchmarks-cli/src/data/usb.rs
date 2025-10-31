use benchmarks_sysinfo::usb::UsbData;
use owo_colors::Style;

use crate::data::{DataProvider, DataRow};

#[derive(Debug, Clone, Default)]
pub struct UsbDataProvider;

impl DataProvider for UsbDataProvider {
    fn identifier(&self) -> &'static str {
        "USB"
    }
    fn try_fetch(&self) -> Result<Vec<super::DataRow>, String> {
        let data = UsbData::fetch().map_err(|err| err.to_string())?;
        let usb_device_count = data.device_ids.len();
        Ok(vec![
            DataRow::new("USB Devices").with_value(format!("{usb_device_count}"), Style::new()),
        ])
    }
}
