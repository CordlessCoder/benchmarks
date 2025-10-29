use crate::Benchmark;

pub struct SystemInformationPanel {}

impl Default for SystemInformationPanel {
    fn default() -> Self {
        SystemInformationPanel {}
    }
}

impl Benchmark for SystemInformationPanel {
    fn name(&self) -> &'static str {
        "System Information"
    }
    fn ui(&mut self, ui: &mut eframe::egui::Ui) {}
}
