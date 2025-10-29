// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::memory::MemoryThroughputPanel;
use eframe::egui;
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
    panel: MemoryThroughputPanel,
}

impl MyApp {
    fn new() -> Self {
        MyApp {
            panel: MemoryThroughputPanel::default(),
        }
    }
}

pub trait Benchmark {
    fn ui(&mut self, ui: &mut egui::Ui);
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("Render stats").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Render time: {:.2}ms",
                    frame.info().cpu_usage.unwrap_or_default() * 1000.0
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_switch(ui);
                })
            })
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Memory Throughput");
                self.panel.ui(ui);
            })
        });
    }
}
