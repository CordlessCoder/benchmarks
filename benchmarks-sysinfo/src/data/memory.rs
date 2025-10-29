use crate::util::{for_colon_separated_line, parse_first_number};

#[derive(Debug, Default)]
pub struct MemInfo {
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub free: u64,
    pub buffers: u64,
    pub cached: u64,
    pub shmem: u64,
    pub s_reclaimable: u64,
}
impl MemInfo {
    pub fn fetch() -> std::io::Result<MemInfo> {
        let mut info = MemInfo::default();
        for_colon_separated_line(
            "/proc/meminfo",
            &mut (),
            |(), name, value| {
                match name {
                    "MemTotal" => {
                        (info.total, _) = parse_first_number(value)?;
                    }
                    "MemAvailable" => {
                        (info.available, _) = parse_first_number(value)?;
                    }
                    "MemFree" => {
                        (info.free, _) = parse_first_number(value)?;
                    }
                    "Buffers" => {
                        (info.buffers, _) = parse_first_number(value)?;
                    }
                    "Cached" => {
                        (info.cached, _) = parse_first_number(value)?;
                    }
                    "Shmem" => {
                        (info.shmem, _) = parse_first_number(value)?;
                    }
                    "SReclaimable" => {
                        (info.s_reclaimable, _) = parse_first_number(value)?;
                    }
                    _ => (),
                }
                Ok(true)
            },
            |(), _| Ok(true),
        )?;
        // Convert KiB -> B
        info.total *= 1024;
        info.available *= 1024;
        info.free *= 1024;
        info.buffers *= 1024;
        info.cached *= 1024;
        info.shmem *= 1024;
        info.s_reclaimable *= 1024;

        if info.available == 0 || info.available >= info.total {
            // MemAvailable can be unreliable
            info.available =
                info.free + info.buffers + info.cached + info.s_reclaimable - info.shmem;
        }
        info.used = info.total.saturating_sub(info.available);
        // Clamp used to <= total
        info.used = info.used.min(info.total);
        Ok(info)
    }
}
