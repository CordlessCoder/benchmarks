use std::{
    fs::File,
    io::{self, BufReader, ErrorKind},
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

use bstr::{ByteSlice, io::BufReadExt};

#[derive(Debug)]
pub struct ChaChaSample {
    pub sampled_at: Instant,
    pub data: ChaChaInstant,
}
#[derive(Debug, Clone, Default)]
pub struct ChaChaInstant {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub bytes: u64,
    pub reads: u64,
    pub writes: u64,
    pub ioctls: u64,
    pub blocks: u64,
    pub buffered_bytes: u64,
    pub errors: u64,
}

static LAST_SAMPLE: LazyLock<Mutex<Option<ChaChaSample>>> = LazyLock::new(|| Mutex::new(None));

impl Drop for ChaChaSample {
    fn drop(&mut self) {
        match &mut *LAST_SAMPLE.lock().unwrap() {
            Some(last) if last.sampled_at < self.sampled_at => {
                core::mem::swap(last, self);
            }
            slot @ None => {
                *slot = Some(core::mem::replace(
                    self,
                    ChaChaSample {
                        sampled_at: self.sampled_at,
                        data: self.data.clone(),
                    },
                ));
            }
            Some(_) => (),
        }
    }
}

pub struct ChaChaDiff {
    pub bytes: u64,
    pub sessions: u64,
    pub over: Duration,
}

impl ChaChaSample {
    pub fn diff_with_last(&self) -> Option<ChaChaDiff> {
        let last_sample = LAST_SAMPLE.lock().unwrap();
        let last_sample = last_sample.as_ref()?;
        let time_diff = self
            .sampled_at
            .saturating_duration_since(last_sample.sampled_at);
        if time_diff.is_zero() {
            return None;
        }
        Some(ChaChaDiff {
            bytes: self.data.bytes - last_sample.data.bytes,
            sessions: self.data.total_sessions - last_sample.data.total_sessions,
            over: time_diff,
        })
    }
    pub fn fetch() -> std::io::Result<Self> {
        let stat = File::open("/proc/chastats")?;
        let mut stat = BufReader::new(stat);
        let mut info = ChaChaSample {
            sampled_at: Instant::now(),
            data: ChaChaInstant::default(),
        };
        stat.for_byte_line(|line| {
            let Some((name, value)) = line.split_once_str(":") else {
                return Ok(false);
            };
            let value = value.trim_ascii();
            let value = core::str::from_utf8(value)
                .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
            let value: u64 = value
                .parse()
                .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
            match name {
                b"Reads" => info.data.reads = value,
                b"Writes" => info.data.writes = value,
                b"Ioctls" => info.data.ioctls = value,
                b"Blocks" => info.data.blocks = value,
                b"Buffer bytes" => info.data.buffered_bytes = value,
                b"Errors" => info.data.errors = value,
                b"Sessions(Active)" => info.data.active_sessions = value,
                b"Sessions(Total)" => info.data.total_sessions = value,
                b"Bytes Processed" => info.data.bytes = value,
                _ => (),
            }
            Ok(true)
        })?;
        Ok(info)
    }
}

pub struct ChaChaData {}
