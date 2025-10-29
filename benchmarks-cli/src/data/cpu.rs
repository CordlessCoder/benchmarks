use crate::data::{DataProvider, DataRow};
use benchmarks_sysinfo::cpu::{CPU, CpuData};
use owo_colors::Style;

#[derive(Debug)]
pub struct CpuDataProvider;
impl DataProvider for CpuDataProvider {
    fn identifier(&self) -> &'static str {
        "CPU"
    }
    fn try_fetch(&self) -> Result<Vec<super::DataRow>, String> {
        let data = CpuData::fetch().map_err(|err| err.to_string())?;
        let mut out: Vec<DataRow> = data
            .cpus
            .into_iter()
            .map(|cpu| {
                let CPU {
                    name,
                    max_freq_khz,
                    cores,
                    threads,
                    features: _,
                } = cpu;
                let mut short_name = String::new();
                for word in name
                    .split(' ')
                    .filter(|word| !word.contains("Processor") && !word.contains("Core"))
                {
                    short_name += word;
                    short_name.push(' ');
                }
                short_name.pop();
                if short_name.is_empty() {
                    short_name = name;
                }
                let thread_text = if cores == threads {
                    format!(" ({threads})")
                } else {
                    format!(" ({cores}/{threads})")
                };
                DataRow::new("CPU")
                    .with_value(short_name, Style::new())
                    .with_value(thread_text, Style::new())
                    .with_value(
                        format!(" @ {:.2} GHz", max_freq_khz as f64 / 1_000_000.0),
                        Style::new(),
                    )
            })
            .collect();
        if out.len() > 1 {
            out.iter_mut().enumerate().for_each(|(i, row)| {
                use core::fmt::Write;
                _ = write!(row.label.to_mut(), " {}", i + 1);
            });
        }
        Ok(out)
    }
}
