use benchmarks_sysinfo::host::HostData;
use owo_colors::Style;

use crate::data::{DataProvider, DataRow};

#[derive(Debug)]
pub struct HostInfoProvider;
impl DataProvider for HostInfoProvider {
    fn identifier(&self) -> &'static str {
        "Host"
    }
    fn try_fetch(&self) -> Result<Vec<super::DataRow>, String> {
        let data = HostData::fetch().map_err(|err| err.to_string())?;
        Ok(vec![
            DataRow::new("Hostname").with_value(data.hostname, Style::new()),
            DataRow::new("Kernel").with_value(data.kernel, Style::new()),
        ])
    }
}
