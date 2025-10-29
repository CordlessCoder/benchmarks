use crate::data::{DataProvider, DataRow};
use benchmarks_sysinfo::network::NetworkData;
use owo_colors::Style;

#[derive(Debug)]
pub struct NetworkProvider;
impl DataProvider for NetworkProvider {
    fn identifier(&self) -> &'static str {
        "Network"
    }
    fn try_fetch(&self) -> Result<Vec<super::DataRow>, String> {
        let data = NetworkData::fetch();
        let rows = data
            .interfaces
            .into_iter()
            .filter(|interface| !interface.is_loopback() && interface.is_up())
            .filter_map(|interface| {
                let addr = interface.ips.first()?.ip().to_string();
                Some(
                    DataRow::new(format!("Local IP ({})", interface.name))
                        .with_value(addr, Style::new()),
                )
            })
            .collect();
        Ok(rows)
    }
}
