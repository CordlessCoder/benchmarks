use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SysinfoRam {
    pub total: u64,
    pub free: u64,
    pub buffer: u64,
    pub shared: u64,
}

#[derive(Debug, Clone)]
pub struct SysInfo {
    pub uptime: Duration,
    pub load_averages: [f32; 3],
    pub ram: SysinfoRam,
    pub processes: u16,
}

impl SysInfo {
    pub fn fetch() -> std::io::Result<Self> {
        let info = unsafe {
            let mut info: libc::sysinfo = core::mem::zeroed();
            if libc::sysinfo(&raw mut info) == -1 {
                return Err(std::io::Error::last_os_error());
            };
            info
        };
        Ok(SysInfo {
            uptime: Duration::from_secs(info.uptime as u64),
            load_averages: info.loads.map(|procs| procs as f32 / 65536.),
            ram: SysinfoRam {
                total: info.totalram,
                buffer: info.bufferram,
                shared: info.sharedram,
                free: info.sharedram,
            },
            processes: info.procs,
        })
    }
}
