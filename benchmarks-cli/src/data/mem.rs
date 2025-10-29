use crate::{
    data::{DataProvider, DataRow},
};
use benchmarks_sysinfo::memory::MemInfo;
use owo_colors::{AnsiColors, Style};
use sizef::IntoSize;

#[derive(Debug)]
pub struct MemDataProvider;
impl DataProvider for MemDataProvider {
    fn identifier(&self) -> &'static str {
        "Memory"
    }
    fn try_fetch(&self) -> Result<Vec<super::DataRow>, String> {
        let info = MemInfo::fetch().map_err(|err| err.to_string())?;
        let used_percent = info.used as f64 * 100.0 / info.total as f64;
        let percent_color = match used_percent {
            0.0..50.0 => AnsiColors::Green,
            50.0..80.0 => AnsiColors::BrightYellow,
            _ => AnsiColors::Red,
        };
        Ok(vec![
            DataRow::new("Memory")
                .with_value(
                    format!(
                        "{} / {} (",
                        info.used.into_decimalsize(),
                        info.total.into_decimalsize()
                    ),
                    Style::new(),
                )
                .with_value(
                    format!("{:.0}%", used_percent),
                    Style::new().color(percent_color),
                )
                .with_value(")", Style::new()),
        ])
    }
}
