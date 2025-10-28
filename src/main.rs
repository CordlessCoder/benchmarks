#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::hash::Hash;

// hide console window on Windows in release
use benchmarks::{
    BenchmarkProgressSnapshop, SelectableEnum,
    impls::{memory::PAGE_SIZE, *},
};
use eframe::egui::{self, ComboBox};
use sizef::IntoSize;

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
    benchmark_config: memory::Config,
    running_benchmark: Option<memory::MemoryThroughputBench>,
    last_progress: Option<BenchmarkProgressSnapshop<memory::State>>,
    total_result: memory::TestResult,
    avg_per_thread_result: memory::TestResult,
}

fn selectable_enum<E: SelectableEnum>(
    ui: &mut egui::Ui,
    id: impl Hash,
    selected: &mut E,
    set_options: impl FnOnce(ComboBox) -> ComboBox,
) {
    set_options(egui::ComboBox::from_id_salt(id))
        .selected_text(selected.as_str())
        .show_ui(ui, |ui| {
            for value in E::all_values() {
                if !value.is_enabled() {
                    continue;
                }
                ui.selectable_value(selected, value.clone(), value.as_str());
            }
        });
}

impl MyApp {
    fn new() -> Self {
        Self {
            benchmark_config: memory::Config {
                passes: 2,
                threads: 1,
                operation: memory::MemoryOperation::Read,
                init_type: memory::MemoryInitializationType::Zeros,
                memory_size: *PAGE_SIZE * 1024 * 10,
                strategy: memory::OperationStrategy::Bytewise,
            },
            running_benchmark: None,
            last_progress: None,
            total_result: memory::TestResult::default(),
            avg_per_thread_result: memory::TestResult::default(),
        }
    }
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
            ui.heading("Memory Throughput");
            egui::Grid::new("memory_benchmark_options").show(ui, |ui| {
                let height = ui.text_style_height(&egui::TextStyle::Body);
                let valign = egui::Align::Max;
                let value_size = [height * 5.5, height * 1.2];
                ui.with_layout(egui::Layout::right_to_left(valign), |ui| {
                    ui.label("Thread(s)");
                });
                ui.add_sized(
                    value_size,
                    egui::DragValue::new(&mut self.benchmark_config.threads)
                        .speed(1)
                        .range(1..=1024),
                );
                ui.end_row();
                ui.with_layout(egui::Layout::right_to_left(valign), |ui| {
                    ui.label("Passes");
                });
                ui.add_sized(
                    value_size,
                    egui::DragValue::new(&mut self.benchmark_config.passes)
                        .speed(1)
                        .range(1..=1024),
                );
                ui.end_row();
                ui.with_layout(egui::Layout::right_to_left(valign), |ui| {
                    ui.label("Memory");
                });
                ui.add_sized(
                    value_size,
                    egui::DragValue::new(&mut self.benchmark_config.memory_size)
                        .speed((*PAGE_SIZE * 16) as f64)
                        .range(
                            *PAGE_SIZE * 4 * self.benchmark_config.threads
                                ..=*PAGE_SIZE * 1024 * 1024 * 32,
                        )
                        .clamp_existing_to_range(true)
                        .custom_formatter(|val, _| val.into_decimalsize().to_string())
                        .custom_parser(|input| {
                            let input = input.trim();
                            let digits = input
                                .bytes()
                                .take_while(|b| matches!(b, b'0'..=b'9' | b'.'))
                                .count();
                            let (number, suffix) = input.split_at(digits);
                            let multiplier = match suffix.trim_start() {
                                "b" | "B" | "byte" => 1.0,
                                "k" | "K" | "kb" | "KB" | "kib" | "KiB" => 1024.0,
                                "" | "m" | "M" | "mb" | "MB" | "mib" | "MiB" => 1024.0 * 1024.0,
                                "g" | "G" | "gb" | "GB" | "gib" | "GiB" => 1024.0 * 1024.0 * 1024.0,
                                _ => return None,
                            };
                            Some(number.parse::<f64>().ok()? * multiplier)
                        }),
                );
                ui.end_row();
                ui.with_layout(egui::Layout::right_to_left(valign), |ui| {
                    ui.label("Operation");
                });
                selectable_enum(
                    ui,
                    "memory_benchmark_option_operation",
                    &mut self.benchmark_config.operation,
                    |ui| ui.width(value_size[0]),
                );
                ui.end_row();
                ui.with_layout(egui::Layout::right_to_left(valign), |ui| {
                    ui.label("Strategy");
                });
                selectable_enum(
                    ui,
                    "memory_benchmark_option_strategy",
                    &mut self.benchmark_config.strategy,
                    |ui| ui.width(value_size[0]),
                );
                ui.end_row();
                ui.with_layout(egui::Layout::right_to_left(valign), |ui| {
                    ui.label("Fill with");
                });
                selectable_enum(
                    ui,
                    "memory_benchmark_option_init_type",
                    &mut self.benchmark_config.init_type,
                    |ui| ui.width(value_size[0]),
                );
                ui.end_row();
                let start_benchmark = ui.add_enabled(
                    self.running_benchmark.is_none(),
                    egui::Button::new("Start benchmark"),
                );
                let cancel_benchmark = ui
                    .with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                        ui.add_enabled(
                            self.running_benchmark.is_some(),
                            egui::Button::new("Cancel"),
                        )
                    })
                    .inner;
                if start_benchmark.clicked() {
                    self.running_benchmark = Some(self.benchmark_config.clone().start())
                }
                if cancel_benchmark.clicked() {
                    self.running_benchmark
                        .as_ref()
                        .unwrap()
                        .progress()
                        .request_stop();
                }
            });
            if let Some(running) = self.running_benchmark.take() {
                let progress = running.progress();
                if running.is_done() {
                    let results = running.wait_for_results();
                    let mut total_result = memory::TestResult::default();
                    for result in results.iter() {
                        total_result.memory_processed += result.memory_processed;
                        total_result.runtime += result.runtime;
                    }
                    total_result.runtime /= results.len().max(1) as u32;
                    let avg_per_thread_result = memory::TestResult {
                        runtime: total_result.runtime,
                        memory_processed: total_result.memory_processed / results.len().max(1),
                    };
                    self.total_result = total_result;
                    self.avg_per_thread_result = avg_per_thread_result;
                } else {
                    self.running_benchmark = Some(running);
                    ctx.request_repaint();
                }
                self.last_progress = Some(progress.load());
            }
            let (progress, stage) = if let Some(progress) = &self.last_progress {
                (
                    progress.as_f32(),
                    if progress.was_cancelled() {
                        "Cancelled".to_string()
                    } else {
                        format!("{}", progress.current_state())
                    },
                )
            } else {
                (0.0, "Not running".to_string())
            };
            ui.horizontal(|ui| {
                ui.add_enabled(
                    self.running_benchmark.is_some(),
                    egui::ProgressBar::new(progress).text(stage),
                );
            });
            ui.heading("Results");
            ui.add_enabled_ui(!self.total_result.runtime.is_zero(), |ui| {
                ui.label("Throughput");
                ui.indent("throughput", |ui| {
                    ui.label(format!(
                        "Total: {}/s",
                        self.total_result.throughput().into_decimalsize()
                    ));
                    ui.label(format!(
                        "Average per thread: {}/s",
                        self.avg_per_thread_result.throughput().into_decimalsize()
                    ));
                });
            });
        });
    }
}
