use crate::{
    Benchmark,
    background_compute::{BackgroundCompute, BackgroundComputeProvider, RepeatedCompute},
};
use benchmarks_sysinfo::{
    chacha::ChaChaSample,
    cpu::{CpuData, CpuUsageSample},
    disk::DiskData,
    host::HostData,
    memory::MemInfo,
    network::NetworkData,
    pci::{PCIData, PciBackendError},
    swap::SwapData,
    sysinfo::SysInfo,
    usb::UsbData,
    user::{PwuIdErr, UserData},
    util::pretty_pci_device::PrettyDevice,
};
use eframe::egui;
use eframe::egui::Sense;
use sizef::IntoSize;
use std::{io, time::Duration};

pub struct SystemInformationPanel {
    cpu: BackgroundCompute<CpuData, std::io::Error>,
    cpu_usage: RepeatedCompute<io::Result<CpuUsageSample>>,
    chacha: RepeatedCompute<io::Result<ChaChaSample>>,
    memory: RepeatedCompute<io::Result<MemInfo>>,
    sysinfo: RepeatedCompute<io::Result<SysInfo>>,
    pci: BackgroundCompute<PCIData, PciBackendError>,
    usb: RepeatedCompute<io::Result<UsbData>>,
    network: RepeatedCompute<io::Result<NetworkData>>,
    disks: RepeatedCompute<io::Result<DiskData>>,
    host: BackgroundCompute<HostData, std::io::Error>,
    user: BackgroundCompute<UserData, PwuIdErr>,
    swap: RepeatedCompute<io::Result<SwapData>>,
    pci_devices_expanded: bool,
    usb_devices_expanded: bool,
}

impl Default for SystemInformationPanel {
    fn default() -> Self {
        SystemInformationPanel {
            cpu: BackgroundCompute::new(CpuData::fetch),
            cpu_usage: RepeatedCompute::new(CpuUsageSample::fetch, Duration::from_secs_f32(0.2)),
            chacha: RepeatedCompute::new(ChaChaSample::fetch, Duration::from_secs_f32(0.2)),
            memory: RepeatedCompute::new(MemInfo::fetch, Duration::from_secs_f32(0.5)),
            sysinfo: RepeatedCompute::new(SysInfo::fetch, Duration::from_secs_f32(0.5)),
            pci: BackgroundCompute::new(PCIData::fetch),
            usb: RepeatedCompute::new(UsbData::fetch, Duration::from_secs(3)),
            network: RepeatedCompute::new(|| Ok(NetworkData::fetch()), Duration::from_secs(5)),
            disks: RepeatedCompute::new(DiskData::fetch, Duration::from_secs(30)),
            host: BackgroundCompute::new(HostData::fetch),
            user: BackgroundCompute::new(UserData::fetch),
            swap: RepeatedCompute::new(SwapData::fetch, Duration::from_secs_f32(0.5)),
            pci_devices_expanded: false,
            usb_devices_expanded: false,
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
                    self.cpu_usage.display(ui, |ui, usage| {
                        let Some(diff) = usage.diff_with_last() else {
                            ui.label("Usage: N/A");
                            return;
                        };
                        ui.label(format!("Usage: {:.2}%", diff.as_usage_factor() * 100.));
                    });
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
        #[cfg(feature = "chacha")]
        {
            ui.heading("ChaCha20");
            self.chacha.display(ui, |ui, chacha| {
                ui.label(format!("Total Sessions: {}", chacha.data.total_sessions));
                ui.label(format!("Active Sessions: {}", chacha.data.active_sessions));
                ui.label(format!(
                    "Data Processed: {}",
                    chacha.data.bytes.into_decimalsize()
                ));
                if let Some(diff) = chacha.diff_with_last() {
                    let throughput = (diff.bytes as f64) / diff.over.as_secs_f64();
                    let sessions_per_second = (diff.sessions as f64) / diff.over.as_secs_f64();
                    ui.label(format!("Throughput: {}/s", throughput.into_decimalsize()));
                    ui.label(format!("Sessions: {sessions_per_second:.1} per second"));
                } else {
                    ui.label("Throughput: N/A");
                    ui.label("Sessions: N/A");
                }
            });
        }

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
        ui.heading("System");
        self.sysinfo.display(ui, |ui, sysinfo| {
            ui.indent("system_indent", |ui| {
                ui.label(format!(
                    "Uptime: {}",
                    humantime::format_duration(sysinfo.uptime)
                ));
                ui.label(format!("Processes: {}", sysinfo.processes));
                ui.label(format!(
                    "Load averages: {:.2} {:.2} {:.2}",
                    sysinfo.load_averages[0], sysinfo.load_averages[1], sysinfo.load_averages[2],
                ));
            });
        });
        ui.heading("Disks");
        self.disks.display(ui, |ui, disks| {
            for disk in &disks.disks {
                ui.label(&disk.device_name);
                ui.indent(("size_for", &disk.device_name), |ui| {
                    ui.label(format!("Model: {}", disk.model));
                    ui.label(format!("Size: {}", disk.size.into_decimalsize()));
                });
            }
        });
        ui.heading("PCI");
        self.pci.display(ui, |ui, pci| {
            ui.indent("pci_list", |ui| {
                ui.heading(if pci.all_devices.len() == 1 {
                    "GPU"
                } else {
                    "GPUs"
                });
                ui.vertical(|ui| {
                    let mut gpu_count: usize = 0;
                    for gpu in pci.gpus() {
                        ui.label(PrettyDevice(gpu).to_string());
                        gpu_count += 1;
                    }

                    let toggle_pci_list = ui.add(
                        egui::Button::new(format!(
                            "Total PCI Devices: {}",
                            pci.all_devices_named.len()
                        ))
                        .selected(self.pci_devices_expanded),
                    );
                    if toggle_pci_list.clicked() {
                        self.pci_devices_expanded = !self.pci_devices_expanded;
                    }
                    let how_expanded = ui.ctx().animate_bool_responsive(
                        ui.id().with("pci_devices_expanded"),
                        self.pci_devices_expanded,
                    );
                    let show_devices = ((pci.all_devices_named.len() - gpu_count) as f32
                        * how_expanded)
                        .ceil() as usize;
                    if show_devices == 0 {
                        return;
                    }
                    pci.all_devices_named
                        .iter()
                        .filter(|dev| !dev.is_gpu)
                        .take(show_devices)
                        .for_each(|device| {
                            ui.label(PrettyDevice(device).to_string());
                        });
                });
            });
        });
        ui.heading("USB");
        self.usb.display(ui, |ui, usb| {
            ui.indent("usb_devices", |ui| {
                let toggle_usb_list = ui.add(
                    egui::Button::new(format!("Total USB Devices: {}", usb.device_ids.len()))
                        .selected(self.usb_devices_expanded),
                );
                if toggle_usb_list.clicked() {
                    self.usb_devices_expanded = !self.usb_devices_expanded;
                }
                let how_expanded = ui.ctx().animate_bool_responsive(
                    ui.id().with("usb_devices_expanded"),
                    self.usb_devices_expanded,
                );
                let show_devices = (usb.device_ids.len() as f32 * how_expanded).ceil() as usize;
                if show_devices == 0 {
                    return;
                }
                usb.device_ids.iter().take(show_devices).for_each(|device| {
                    ui.label(format!(
                        "{}, {}",
                        device.product.as_deref().unwrap_or("Unknown"),
                        device
                            .manufacturer
                            .as_deref()
                            .unwrap_or("Unknown Manufacturer"),
                    ));
                });
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
                        if interface.ips.is_empty() {
                            return;
                        }
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
