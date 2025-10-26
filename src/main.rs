#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::{
    sync::Arc,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use benchmarks::BenchmarkProgressTracker;
use eframe::egui::{self, Atom, Atoms};

fn main() -> eframe::Result {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| {
            // // This gives us image support:
            // egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(MyApp::new()))
        }),
    )
}

struct MyApp {
    progress: Arc<BenchmarkProgressTracker>,
    working_thread: Option<JoinHandle<()>>,
}

impl MyApp {
    fn new() -> Self {
        Self {
            progress: Arc::new(BenchmarkProgressTracker::new(1024, 1)),
            working_thread: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.progress.all_threads_done()
                && let Some(thread) = self.working_thread.take() {
                    _ = thread.join();
                }
            let start_benchmark = ui.add_enabled(
                self.working_thread.is_none(),
                egui::Button::new("Start benchmark."),
            );
            if start_benchmark.clicked() {
                let work_units = 1024;
                self.progress.reset(work_units, 1);
                let thread_progress = Arc::clone(&self.progress);
                self.working_thread = Some(std::thread::spawn(move || {
                    for _ in 0..work_units {
                        thread_progress.add(1);
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    thread_progress.flag_thread_done();
                }));
            }
            if self.working_thread.is_some() {
                ctx.request_repaint_after(Duration::from_millis(10));
            }
            ui.add(egui::ProgressBar::new(self.progress.load().as_f32()));
            ui.label(format!(
                "Render time: {:.2}ms",
                frame.info().cpu_usage.unwrap_or_default() * 1000.0
            ));
        });
    }
}
