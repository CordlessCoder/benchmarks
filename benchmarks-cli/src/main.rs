// #![cfg_attr(not(feature = "tracing"), expect(clippy::unnecessary_lazy_evaluations))]
mod cli;
use clap::Parser;
use owo_colors::{OwoColorize, Stream, Style};
use tracing_subscriber::{EnvFilter, fmt, fmt::format::FmtSpan, prelude::*};

use benchmarks_cli::data::{
    DataProvider, DataRow, StyledText, cpu::CpuDataProvider, disk::DiskDataProvider,
    gpu::GpuDataProvider, host::HostInfoProvider, ip::NetworkProvider, mem::MemDataProvider,
    pci_totals::PciTotalProvider, swap::SwapDataProvider, uptime::UptimeProvider,
    usb::UsbDataProvider, user::UserInfoProvider,
};

// TODO: Add
// [ ] Disk provider
static ALL_PROVIDERS: &[&dyn DataProvider] = &[
    &CpuDataProvider,
    &MemDataProvider,
    &GpuDataProvider,
    &PciTotalProvider,
    &NetworkProvider,
    &UsbDataProvider,
    &DiskDataProvider,
    &SwapDataProvider,
    &HostInfoProvider,
    &UptimeProvider,
    &UserInfoProvider,
];

pub const ERROR_STYLE: Style = Style::new().red().bold();
pub const LABEL_STYLE: Style = Style::new().blue().bold();

fn main() {
    let args = cli::Args::parse();
    if args.print_identifiers {
        for provider in ALL_PROVIDERS {
            println!("{}", provider.identifier());
        }
        return;
    }
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_span_events(match args.verbose {
                    0 => FmtSpan::CLOSE,
                    1 => FmtSpan::ENTER | FmtSpan::CLOSE,
                    2.. => FmtSpan::FULL,
                })
                .with_timer(tracing_subscriber::fmt::time::uptime())
                .pretty(),
        )
        .with(EnvFilter::from_default_env())
        .init();

    for &provider in ALL_PROVIDERS {
        let ident = provider.identifier();
        if args
            .disable
            .iter()
            .any(|disabled| disabled.eq_ignore_ascii_case(ident))
        {
            continue;
        }
        let _span = tracing::info_span!(
            "Using Data Provider",
            ?provider,
            // name = provider.identifier()
        );
        let rows = match provider.try_fetch() {
            Ok(rows) => rows,
            Err(err) => {
                eprintln!(
                    "{}",
                    err.if_supports_color(Stream::Stderr, |text| text.style(ERROR_STYLE))
                );
                continue;
            }
        };
        for DataRow { label, values } in rows {
            print!(
                "{}: ",
                label.if_supports_color(Stream::Stdout, |text| text.style(LABEL_STYLE))
            );
            for StyledText { style, text } in values {
                print!(
                    "{}",
                    text.if_supports_color(Stream::Stdout, |text| text.style(style))
                );
            }
            println!();
        }
    }
}
