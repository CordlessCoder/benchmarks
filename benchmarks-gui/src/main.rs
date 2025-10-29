// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use crate::{information::SystemInformationPanel, memory::MemoryThroughputPanel};
use eframe::egui;
mod background_compute;
mod information;
mod memory;

fn main() -> eframe::Result {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        ..Default::default()
    };
    eframe::run_native(
        "Benchmarks",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}

struct MyApp {
    benchmarks: Vec<Box<dyn Benchmark>>,
    selector_panel_open: bool,
    selected_benchmark_idx: Option<usize>,
}

impl MyApp {
    fn new() -> Self {
        MyApp {
            benchmarks: vec![
                Box::new(SystemInformationPanel::default()),
                Box::new(MemoryThroughputPanel::default()),
            ],
            selected_benchmark_idx: Some(0),
            selector_panel_open: true,
        }
    }
}

pub trait Benchmark {
    fn ui(&mut self, ui: &mut egui::Ui);
    fn name(&self) -> &'static str;
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("Render stats").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                egui::widgets::global_theme_preference_switch(ui);
                ui.separator();
                ui.toggle_value(&mut self.selector_panel_open, "Benchmark List");
                ui.separator();
                ui.label(format!(
                    "Render time: {:.2}ms",
                    frame.info().cpu_usage.unwrap_or_default() * 1000.0
                ));
            })
        });
        egui::SidePanel::left("Benchmark selector")
            .resizable(false)
            .show_animated(ctx, self.selector_panel_open, |ui| {
                ui.add_space(ui.spacing().menu_margin.topf());
                for (idx, benchmark) in self.benchmarks.iter().enumerate() {
                    ui.selectable_value(
                        &mut self.selected_benchmark_idx,
                        Some(idx),
                        benchmark.name(),
                    );
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.take_available_space();
                let Some(selected) = self
                    .selected_benchmark_idx
                    .and_then(|idx| self.benchmarks.get_mut(idx))
                else {
                    return;
                };
                selected.ui(ui);
            })
        });
    }
}
