use crate::{Benchmark, background_compute::BackgroundCompute};
use benchmarks_sysinfo::{
    cpu::CpuData,
    host::HostData,
    memory::MemInfo,
    network::NetworkData,
    pci::{PCIData, PciBackendError},
    swap::SwapData,
    user::{PwuIdErr, UserData},
    util::PrettyDevice,
};
use eframe::egui;
use eframe::egui::Sense;
use sizef::IntoSize;
use std::convert::Infallible;

pub struct SystemInformationPanel {
    cpu: BackgroundCompute<CpuData, std::io::Error>,
    memory: BackgroundCompute<MemInfo, std::io::Error>,
    pci: BackgroundCompute<PCIData, PciBackendError>,
    network: BackgroundCompute<NetworkData, Infallible>,
    host: BackgroundCompute<HostData, std::io::Error>,
    user: BackgroundCompute<UserData, PwuIdErr>,
    swap: BackgroundCompute<SwapData, std::io::Error>,
}

impl Default for SystemInformationPanel {
    fn default() -> Self {
        SystemInformationPanel {
            cpu: BackgroundCompute::new(CpuData::fetch),
            memory: BackgroundCompute::new(MemInfo::fetch),
            pci: BackgroundCompute::new(PCIData::fetch),
            network: BackgroundCompute::new(|| Ok(NetworkData::fetch())),
            host: BackgroundCompute::new(HostData::fetch),
            user: BackgroundCompute::new(UserData::fetch),
            swap: BackgroundCompute::new(SwapData::fetch),
        }
    }
}

impl Benchmark for SystemInformationPanel {
    fn name(&self) -> &'static str {
        "System Information"
    }
    fn ui(&mut self, ui: &mut eframe::egui::Ui) {
        ui.heading("CPU");
        self.cpu.display(ui, |ui, cpu| {
            for cpu in &cpu.cpus {
                ui.label(&cpu.name);
                ui.indent("cpu_indent", |ui| {
                    ui.label(format!(
                        "Max frequency: {:.2} GHz",
                        cpu.max_freq_khz as f64 / 1_000_000.0
                    ));
                    ui.label(format!("Cores: {}", cpu.cores));
                    ui.label(format!("Threads: {}", cpu.threads));
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Features:");
                        #[allow(unused)]
                        let mut untoggeable_label = |enabled: bool, name: &str| {
                            ui.add(egui::Button::selectable(enabled, name).sense(Sense::empty()))
                        };
                        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                        untoggeable_label(cpu.features.sse, "SSE");
                        #[cfg(target_arch = "x86_64")]
                        untoggeable_label(cpu.features.avx2, "AVX2");
                        #[cfg(target_arch = "x86_64")]
                        untoggeable_label(cpu.features.avx512, "AVX512");
                    });
                });
            }
        });
        ui.heading("Memory");
        self.memory.display(ui, |ui, mem| {
            ui.indent("memory_indent", |ui| {
                ui.label(format!("Total: {}", mem.total.into_decimalsize()));
                ui.label(format!(
                    "Used: {} ({:.1}%)",
                    mem.used.into_decimalsize(),
                    mem.used as f64 * 100.0 / mem.total as f64
                ));
            });
        });
        ui.heading("PCI");
        self.pci.display(ui, |ui, pci| {
            ui.indent("pci_list", |ui| {
                ui.heading(if pci.gpus.len() == 1 { "GPU" } else { "GPUs" });
                ui.vertical(|ui| {
                    for gpu in &pci.gpus {
                        ui.label(PrettyDevice(gpu).to_string());
                    }
                });
                ui.label(format!("Total PCI Devices: {}", pci.all_devices.len()));
            });
        });
        ui.heading("Network");
        ui.indent("network_indent", |ui| {
            self.network.display(ui, |ui, net| {
                for interface in &net.interfaces {
                    ui.add_enabled_ui(interface.is_running(), |ui| {
                        ui.label(format!(
                            "Interface {} {}",
                            interface.name, interface.description
                        ));
                        ui.label(if interface.ips.len() == 1 {
                            "Address"
                        } else {
                            "Addresses"
                        });
                        ui.indent(("interface_addresses", interface.index), |ui| {
                            for addr in &interface.ips {
                                ui.label(addr.to_string());
                            }
                        });
                    });
                }
            });
        });
        ui.heading("Host");
        self.host.display(ui, |ui, HostData { hostname, kernel }| {
            ui.indent("host_indent", |ui| {
                ui.label(format!("Hostname: {hostname}"));
                ui.label(format!("Kernel: {kernel}"));
            });
        });
        ui.heading("User");
        self.user.display(ui, |ui, data| {
            ui.indent("user_indent", |ui| {
                let UserData {
                    username,
                    home,
                    shell,
                } = data;
                ui.label(format!("User: {username}"));
                ui.label(format!("Home: {}", home.display()));
                ui.label(format!("Shell: {}", shell.name()));
                ui.indent("shell_indent", |ui| {
                    ui.label(format!("Path: {}", shell.path.display()));
                    if let Some(version) = shell.version() {
                        ui.label(format!("Version: {version}"));
                    }
                });
            });
        });
        ui.heading("Swap");
        self.swap.display(ui, |ui, swap| {
            ui.indent("swap_indent", |ui| {
                for swap in &swap.swaps {
                    ui.label(format!("Path: {}", swap.name.display()));
                    ui.indent("swap_indent", |ui| {
                        ui.label(format!("Size: {}", swap.size.into_decimalsize()));
                        ui.label(format!(
                            "Used: {} ({:.1}%)",
                            swap.used.into_decimalsize(),
                            swap.used as f64 * 100.0 / swap.size as f64
                        ));
                    });
                }
            });
        });
    }
}
