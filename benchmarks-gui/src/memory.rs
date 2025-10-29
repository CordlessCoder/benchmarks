use benchmarks_memory as memory;
// hide console window on Windows in release
use benchmarks_core::{BenchmarkProgressSnapshop, selectable_enum};
use eframe::{egui, emath::Float};
use memory::PAGE_SIZE;
use sizef::IntoSize;

use crate::Benchmark;

pub struct MemoryThroughputPanel {
    benchmark_config: memory::Config,
    running_benchmark: Option<memory::MemoryThroughputBench>,
    last_progress: Option<BenchmarkProgressSnapshop<memory::State>>,
    total_result: memory::TestResult,
    avg_per_thread_result: memory::TestResult,
    min_per_thread_result: memory::TestResult,
    max_per_thread_result: memory::TestResult,
}

impl Default for MemoryThroughputPanel {
    fn default() -> Self {
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
            min_per_thread_result: memory::TestResult::default(),
            max_per_thread_result: memory::TestResult::default(),
        }
    }
}

impl MemoryThroughputPanel {
    fn draw_options(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("memory_benchmark_options").show(ui, |ui| {
            let height = ui.text_style_height(&egui::TextStyle::Body);
            let valign = egui::Align::Max;
            let value_size = [height * 5.5, height * 1.2];
            ui.with_layout(egui::Layout::right_to_left(valign), |ui| {
                ui.label("Thread(s)")
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
        });
    }
    fn draw_results(&mut self, ui: &mut egui::Ui) {
        ui.heading("Results");
        ui.add_enabled_ui(!self.total_result.runtime.is_zero(), |ui| {
            egui::Grid::new("memory_benchmark_results").show(ui, |ui| {
                ui.label("Total:");
                ui.label(format!(
                    "{}/s",
                    self.total_result.throughput().into_decimalsize()
                ));
                ui.end_row();
                ui.label("Average thread:");
                ui.label(format!(
                    "{}/s",
                    self.avg_per_thread_result.throughput().into_decimalsize()
                ));
                ui.end_row();
                ui.label("Slowest thread:");
                ui.label(format!(
                    "{}/s",
                    self.min_per_thread_result.throughput().into_decimalsize()
                ));
                ui.end_row();
                ui.label("Fastest thread:");
                ui.label(format!(
                    "{}/s",
                    self.max_per_thread_result.throughput().into_decimalsize()
                ));
            })
        });
    }
    fn draw_start_button(&mut self, ui: &mut egui::Ui) {
        let start_benchmark = ui.add_enabled(
            self.running_benchmark.is_none(),
            egui::Button::new("Start benchmark"),
        );
        if start_benchmark.clicked() {
            self.running_benchmark = Some(self.benchmark_config.clone().start());
        }
        if self.running_benchmark.is_some() {
            // Draw the Cancel button over top of the Start benchmark button, this is fine as
            // this will only happen if the start button was disabled anyway.
            let cancel_benchmark = ui.put(start_benchmark.rect, egui::Button::new("Cancel"));
            if cancel_benchmark.clicked() {
                self.running_benchmark
                    .as_ref()
                    .unwrap()
                    .progress()
                    .request_stop();
            }
        }
    }
    fn update_progress(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        if let Some(running) = self.running_benchmark.take() {
            let progress = running.progress();
            if running.is_done() {
                let results = running.wait_for_results();
                self.min_per_thread_result = results
                    .iter()
                    .min_by_key(|r| r.throughput().ord())
                    .copied()
                    .unwrap_or_default();
                self.max_per_thread_result = results
                    .iter()
                    .max_by_key(|r| r.throughput().ord())
                    .copied()
                    .unwrap_or_default();
                let mut total_result = memory::TestResult::default();
                for result in &results {
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
    }
    fn draw_progress_bar(&mut self, ui: &mut egui::Ui) {
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
        ui.add_enabled(
            self.running_benchmark.is_some(),
            egui::ProgressBar::new(progress).text(stage),
        );
    }
}

impl Benchmark for MemoryThroughputPanel {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.draw_options(ui);
            ui.separator();
            ui.vertical(|ui| {
                self.draw_results(ui);
            })
        });
        ui.separator();
        ui.horizontal(|ui| {
            self.draw_start_button(ui);
            self.update_progress(ui);
            self.draw_progress_bar(ui);
        });
    }
}
