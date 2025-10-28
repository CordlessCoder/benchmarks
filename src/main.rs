#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use benchmarks::{BenchmarkProgressSnapshop, impls::memory::PAGE_SIZE, impls::*};
use eframe::egui;
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
        Box::new(|_cc| {
            // // This gives us image support:
            // egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(MyApp::new()))
        }),
    )
}

struct MyApp {
    benchmark_config: memory::Config,
    running_benchmark: Option<memory::MemoryThroughputBench>,
    last_progress: Option<BenchmarkProgressSnapshop<memory::State>>,
    total_result: memory::TestResult,
    avg_per_thread_result: memory::TestResult,
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
                strategy: memory::OperationStrategy::Generic,
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
                let value_size = [height * 4.0, height];
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
                        .range(*PAGE_SIZE * 4 * self.benchmark_config.threads..=*PAGE_SIZE * 1024 * 1024 * 32)
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
                                "" | "b" | "B" | "byte" => 1.0,
                                "k" | "K" | "kb" | "KB" | "kib" | "KiB" => 1024.0,
                                "m" | "M" | "mb" | "MB" | "mib" | "MiB" => 1024.0 * 1024.0,
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
                egui::ComboBox::from_id_salt("memory_benchmark_option_operation")
                    .selected_text(format!("{:?}", self.benchmark_config.operation))
                    .width(value_size[0])
                    .show_ui(ui, |ui| {
                        use memory::MemoryOperation::*;
                        let op = &mut self.benchmark_config.operation;
                        ui.selectable_value(op, Read, "Read");
                        ui.selectable_value(op, Write, "Write");
                        ui.selectable_value(op, Copy, "Copy");
                    });
                ui.end_row();
                ui.with_layout(egui::Layout::right_to_left(valign), |ui| {
                    ui.label("Strategy");
                });
                egui::ComboBox::from_id_salt("memory_benchmark_option_strategy")
                    .selected_text(format!("{:?}", self.benchmark_config.strategy))
                    .width(value_size[0])
                    .show_ui(ui, |ui| {
                        use memory::OperationStrategy::*;
                        let op = &mut self.benchmark_config.strategy;
                        ui.selectable_value(op, Generic, "Generic");
                        ui.selectable_value(op, Int32, "Int32");
                        ui.selectable_value(op, Int64, "Int64");
                        ui.selectable_value(op, Int128, "Int128");
                        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                        ui.selectable_value(op, SSE, "SSE");
                        #[cfg(target_arch = "x86_64")]
                        ui.selectable_value(op, AVX2, "AVX2");
                        #[cfg(target_arch = "x86_64")]
                        ui.selectable_value(op, AVX512, "AVX512");
                    });
                ui.end_row();
                ui.with_layout(egui::Layout::right_to_left(valign), |ui| {
                    ui.label("Fill with");
                });
                egui::ComboBox::from_id_salt("memory_benchmark_option_init_type")
                    .selected_text(format!("{:?}", self.benchmark_config.init_type))
                    .width(value_size[0])
                    .show_ui(ui, |ui| {
                        use memory::MemoryInitializationType::*;
                        let init = &mut self.benchmark_config.init_type;
                        ui.selectable_value(init, Zeros, "Zeros");
                        ui.selectable_value(init, Ones, "Ones");
                        ui.selectable_value(init, Random, "Random");
                    });
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
                    .response;
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
