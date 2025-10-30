use crate::util::for_colon_separated_line;
use bstr::{ByteSlice, io::BufReadExt};
use core::fmt::Write;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, Read},
    path::PathBuf,
    str::FromStr,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
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

#[derive(Debug)]
pub struct CpuUsageSample {
    pub sampled_at: Instant,
    pub cores: Vec<CoreUsageSample>,
}
#[derive(Debug)]
pub struct CoreUsageSample {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
    pub core: Option<u16>,
}

fn parse_first_number_bytes<T: FromStr>(data: &mut &[u8]) -> Result<Option<T>, T::Err> {
    *data = data.trim_start();
    let digits = data.iter().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 {
        return Ok(None);
    };
    let (number, rest) = data.split_at(digits);
    let number = unsafe { core::str::from_utf8_unchecked(number) };
    *data = rest;
    Some(number.parse()).transpose()
}

impl CoreUsageSample {
    pub fn get_idle_jiffies(&self) -> u64 {
        self.idle + self.iowait
    }
    pub fn get_total_jiffies(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }
    fn parse_line(line: &[u8]) -> std::io::Result<Option<CoreUsageSample>> {
        let Some(mut cpu_line) = line.strip_prefix(b"cpu") else {
            return Ok(None);
        };
        let core_num = cpu_line
            .first()
            .is_some_and(|b| b.is_ascii_digit())
            .then_some(())
            .and_then(|_| parse_first_number_bytes::<u16>(&mut cpu_line).transpose())
            .transpose()
            .map_err(|e| std::io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut data = [0; 8];
        for entry in &mut data {
            let value = parse_first_number_bytes::<u64>(&mut cpu_line)
                .map_err(|e| std::io::Error::new(io::ErrorKind::InvalidData, e))?
                .ok_or_else(|| {
                    std::io::Error::new(io::ErrorKind::InvalidData, "Couldn't find expected entry")
                })?;
            *entry = value;
        }
        let [user, nice, system, idle, iowait, irq, softirq, steal] = data;
        Ok(Some(CoreUsageSample {
            user,
            nice,
            system,
            idle,
            iowait,
            irq,
            softirq,
            steal,
            core: core_num,
        }))
    }
}

impl CpuUsageSample {
    pub fn diff_with_last(&self) -> Option<CpuUsageDiff> {
        let last_sample = LAST_SAMPLE.lock().unwrap();
        let last_sample = last_sample.as_ref()?;
        let time_diff = self
            .sampled_at
            .saturating_duration_since(last_sample.sampled_at);
        if time_diff.is_zero() {
            return None;
        }
        // Get global usage
        let last_global = last_sample.cores.iter().find(|c| c.core.is_none())?;
        let current_global = self.cores.iter().find(|c| c.core.is_none())?;
        let total_diff = current_global.get_total_jiffies() - last_global.get_total_jiffies();
        let idle_diff = current_global.get_idle_jiffies() - last_global.get_idle_jiffies();
        Some(CpuUsageDiff {
            total: total_diff,
            idle: idle_diff,
            over: time_diff,
        })
    }
    pub fn fetch() -> std::io::Result<CpuUsageSample> {
        let stat = File::open("/proc/stat")?;
        let mut stat = BufReader::new(stat);
        let mut info = CpuUsageSample {
            sampled_at: Instant::now(),
            cores: Vec::new(),
        };
        stat.for_byte_line(|line| {
            let sample = CoreUsageSample::parse_line(line)?;
            if let Some(sample) = sample {
                info.cores.push(sample);
            }
            Ok(true)
        })?;
        Ok(info)
    }
}

#[derive(Debug)]
pub struct CpuUsageDiff {
    pub total: u64,
    pub idle: u64,
    pub over: Duration,
}

impl CpuUsageDiff {
    pub fn as_usage_factor(&self) -> f32 {
        1.0 - self.idle as f32 / self.total as f32
    }
}

impl Drop for CpuUsageSample {
    fn drop(&mut self) {
        if self.cores.is_empty() {
            return;
        }
        match &mut *LAST_SAMPLE.lock().unwrap() {
            Some(last) if last.sampled_at < self.sampled_at => {
                core::mem::swap(last, self);
            }
            slot @ None => {
                let empty = CpuUsageSample {
                    sampled_at: self.sampled_at,
                    cores: Vec::new(),
                };
                *slot = Some(core::mem::replace(self, empty));
            }
            Some(_) => (),
        }
    }
}

static LAST_SAMPLE: LazyLock<Mutex<Option<CpuUsageSample>>> = LazyLock::new(|| Mutex::new(None));

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
