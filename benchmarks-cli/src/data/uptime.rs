use benchmarks_sysinfo::sysinfo::SysInfo;
use owo_colors::Style;

use crate::data::{DataProvider, DataRow};

#[derive(Debug, Clone, Default)]
pub struct UptimeProvider;

impl DataProvider for UptimeProvider {
    fn identifier(&self) -> &'static str {
        "Uptime"
    }
    fn try_fetch(&self) -> Result<Vec<super::DataRow>, String> {
        let info = SysInfo::fetch().map_err(|err| err.to_string())?;
        Ok(vec![DataRow::new("Uptime").with_value(
            format!("{}", humantime::format_duration(info.uptime)),
            Style::new(),
        )])
    }
}
