use nix::{fcntl::OFlag, sys::stat::Mode};
use rxfetch::cached_path::CachedPath;
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};
use tracing::debug;

const SYS_BLOCK: &str = "/sys/block/";

pub struct DiskData {
    pub disks: Vec<Disk>,
}

pub struct Disk {
    pub model: String,
    pub device_name: String,
    pub size: u64,
}

fn parse_u64_from_path(path: &Path) -> io::Result<u64> {
    let mut buf = [0; u64::MAX.ilog10() as usize + 1];
    let mut size_file = match File::open(path) {
        Ok(size) => size,
        Err(err) => {
            return Err(err);
        }
    };
    let mut consumed = 0;
    while consumed != buf.len() {
        let got = size_file.read(&mut buf[consumed..])?;
        if got == 0 {
            break;
        }
        consumed += got;
    }
    let digits = buf.iter().copied().take_while(u8::is_ascii_digit).count();
    let size = &buf[..digits];
    // SAFETY: All bytes of size are ascii digits, so they must be valid UTF-8
    let size = unsafe { core::str::from_utf8_unchecked(size) };
    size.parse()
        .map_err(|_err| io::Error::new(io::ErrorKind::InvalidData, "Could not parse size of disk"))
}

impl Disk {
    pub fn get_from_path(path: &mut PathBuf, device_name: String) -> io::Result<Self> {
        path.push("device/model");
        let mut model = match std::fs::read_to_string(&path) {
            Ok(model) => model,
            Err(err) => {
                path.pop();
                path.pop();
                return Err(err);
            }
        };
        path.pop();
        path.pop();
        while model.chars().last().is_some_and(|c| c.is_whitespace()) {
            model.pop();
        }
        path.push("size");
        let size_in_blocks = parse_u64_from_path(path);
        path.pop();
        let size_in_blocks = size_in_blocks?;
        path.push("queue/logical_block_size");
        let block_size = parse_u64_from_path(path).ok().unwrap_or(512);
        path.pop();
        path.pop();
        Ok(Disk {
            model,
            size: size_in_blocks * block_size,
            device_name,
        })
    }
}

impl DiskData {
    pub fn fetch() -> io::Result<Self> {
        let block_dir =
            nix::dir::Dir::open(SYS_BLOCK, OFlag::O_RDONLY | OFlag::O_CLOEXEC, Mode::empty())?;
        let mut disks = Vec::new();
        let mut path = CachedPath::take();
        path.clear();
        path.push(SYS_BLOCK);
        for block_device in block_dir {
            let block_device = block_device?;
            let path_name = block_device.file_name().to_bytes();
            if matches!(path_name, b"." | b"..") {
                continue;
            }
            let device_name = core::str::from_utf8(path_name)
                .map_err(|_err| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Failed to parse block device name as UTF-8",
                    )
                })?
                .to_string();
            path.push(&device_name);
            match Disk::get_from_path(&mut path, device_name) {
                Ok(disk) => disks.push(disk),
                Err(err) => debug!("Failed to get disk info: {err}"),
            }
            path.pop();
        }
        Ok(DiskData { disks })
    }
}
