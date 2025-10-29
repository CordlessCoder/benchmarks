use crate::{
    data::{DataProvider, DataRow},
};
use benchmarks_sysinfo::swap::{Swap, SwapData};
use owo_colors::{AnsiColors, Style};
use sizef::IntoSize;

#[derive(Debug)]
pub struct SwapDataProvider;
impl DataProvider for SwapDataProvider {
    fn identifier(&self) -> &'static str {
        "Memory"
    }
    fn try_fetch(&self) -> Result<Vec<super::DataRow>, String> {
        let data = SwapData::fetch().map_err(|err| err.to_string())?;
        Ok(data
            .swaps
            .into_iter()
            .map(|Swap { size, used, .. }| {
                let used_percent = used as f64 / size as f64;
                let percent_color = match used_percent {
                    0.0..50.0 => AnsiColors::Green,
                    50.0..80.0 => AnsiColors::BrightYellow,
                    _ => AnsiColors::Red,
                };
                DataRow::new("Swap")
                    .with_value(
                        format!(
                            "{used} / {size} (",
                            used = used.into_decimalsize(),
                            size = size.into_decimalsize(),
                        ),
                        Style::new(),
                    )
                    .with_value(
                        format!("{used_percent:.0}%"),
                        Style::new().color(percent_color),
                    )
                    .with_value(")", Style::new())
            })
            .collect())
    }
}
