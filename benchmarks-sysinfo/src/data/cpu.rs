// TODO: Support CPU Usage computation by polling /proc/stat
use crate::util::for_colon_separated_line;
use core::fmt::Write;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::PathBuf,
};
use tracing::warn;

#[derive(Debug, Default, Clone)]
struct Core {
    processor: u16,
    core_id: u16,
    bogomips: f32,
    name: String,
    freq_mhz: f32,
    physical_id: u16,
    features: CPUFeatures,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CPUFeatures {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub sse: bool,
    #[cfg(target_arch = "x86_64")]
    pub avx2: bool,
    #[cfg(target_arch = "x86_64")]
    pub avx512: bool,
}

#[derive(Debug, Clone)]
pub struct CPU {
    pub name: String,
    pub max_freq_khz: u32,
    pub cores: u16,
    pub threads: u16,
    pub features: CPUFeatures,
}

#[derive(Debug, Clone)]
pub struct CpuData {
    pub cpus: Vec<CPU>,
}

impl CpuData {
    pub fn fetch() -> io::Result<Self> {
        let mut cores: Vec<Core> = Vec::new();
        let mut current_core = Core::default();
        for_colon_separated_line(
            "/proc/cpuinfo",
            &mut current_core,
            |current_core, name, value| {
                match name {
                    "physical id" => {
                        let Ok(id): Result<u16, _> = value.parse() else {
                            warn!(name: "Failed to parse CPUINFO physical id", value);
                            return Ok(true);
                        };
                        current_core.physical_id = id;
                    }
                    "core id" => {
                        let Ok(core_id): Result<u16, _> = value.parse() else {
                            warn!(name: "Failed to parse CPUINFO processor number", value);
                            return Ok(true);
                        };
                        current_core.core_id = core_id;
                    }
                    "processor" => {
                        let Ok(processor): Result<u16, _> = value.parse() else {
                            warn!(name: "Failed to parse CPUINFO processor number", value);
                            return Ok(true);
                        };
                        current_core.processor = processor;
                    }
                    "flags" => {
                        if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
                            current_core.features.sse = value.contains("sse");
                        }
                        if cfg!(target_arch = "x86_64") {
                            current_core.features.avx2 = value.contains("avx2");
                        }
                        if cfg!(target_arch = "x86_64") {
                            current_core.features.avx512 = value.contains("avx512f");
                        }
                    }
                    "bogomips" => {
                        let Ok(bogomips): Result<f32, _> = value.parse() else {
                            warn!(name: "Failed to parse CPUINFO bogomips number", value);
                            return Ok(true);
                        };
                        current_core.bogomips = bogomips;
                    }
                    "model name" => {
                        current_core.name = value.to_string();
                    }
                    "cpu MHz" => {
                        let Ok(freq_mhz): Result<f32, _> = value.parse() else {
                            warn!(name: "Failed to parse CPUINFO cpu MHz number", value);
                            return Ok(true);
                        };
                        current_core.freq_mhz = freq_mhz;
                    }
                    _ => (),
                }
                Ok(true)
            },
            |current_core, _| {
                cores.push(core::mem::take(current_core));
                Ok(true)
            },
        )?;
        let mut last_id = cores.first().map(|c| c.physical_id).unwrap_or_default();
        let mut cpus: Vec<CPU> = Vec::new();
        let mut last_name = String::new();
        let mut path_buf = PathBuf::new();
        let mut sum_max_freq = 0;
        let mut max_freq_count = 0;
        let mut thread_per_core_counts: HashMap<u16, u16> = HashMap::new();
        let mut features = CPUFeatures::default();
        for core in cores {
            *thread_per_core_counts.entry(core.core_id).or_default() += 1;
            let storage = path_buf.as_mut_os_string();
            storage.clear();
            _ = write!(
                storage,
                "/sys/devices/system/cpu/cpufreq/policy{core}/cpuinfo_max_freq",
                core = core.processor
            );
            'freq: {
                let Ok(mut max_freq) = File::open(&path_buf) else {
                    break 'freq;
                };
                let mut buf = [0; 32];
                let mut read = 0;
                loop {
                    let got = max_freq.read(&mut buf[read..]).unwrap();
                    if got == 0 {
                        break;
                    }
                    read += got;
                    if read == buf.len() {
                        break;
                    }
                }
                let text = &buf[..read].trim_ascii();
                if !text.iter().all(u8::is_ascii_digit) {
                    break 'freq;
                }
                let text = core::str::from_utf8(text).unwrap();
                let Ok(freq) = text.parse::<u32>() else {
                    break 'freq;
                };
                sum_max_freq += freq;
                max_freq_count += 1;
            }
            features = core.features;
            if core.physical_id != last_id {
                cpus.push(CPU {
                    name: core::mem::take(&mut last_name),
                    max_freq_khz: sum_max_freq / max_freq_count.max(1),
                    cores: thread_per_core_counts.len() as u16,
                    threads: thread_per_core_counts.values().sum(),
                    features: core::mem::take(&mut features),
                });
                thread_per_core_counts.clear();
                last_id = core.physical_id;
                max_freq_count = 0;
                sum_max_freq = 0;
            }
            last_name = core.name;
        }
        cpus.push(CPU {
            name: core::mem::take(&mut last_name),
            max_freq_khz: sum_max_freq / max_freq_count.max(1),
            cores: thread_per_core_counts.len() as u16,
            threads: thread_per_core_counts.values().sum(),
            features,
        });
        Ok(Self { cpus })
    }
}
