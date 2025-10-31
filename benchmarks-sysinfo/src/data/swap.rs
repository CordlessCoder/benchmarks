use crate::util::parse_from_bytes;
use bstr::io::BufReadExt;
use rxfetch::display::DisplayBytes;
use std::{
    ffi::OsString,
    fs::File,
    io::{BufReader, Error, ErrorKind},
    os::unix::ffi::OsStringExt,
    path::PathBuf,
};

#[derive(Debug, Clone)]
pub struct Swap {
    pub name: PathBuf,
    pub size: u64,
    pub used: u64,
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub struct SwapData {
    pub swaps: Vec<Swap>,
}

impl Swap {
    fn from_line(line: &[u8]) -> std::io::Result<Self> {
        let mut words = line
            .split(|b| b.is_ascii_whitespace())
            .filter(|segment| !segment.is_empty());
        let [
            Some(name),
            Some(_type),
            Some(size),
            Some(used),
            Some(priority),
        ] = core::array::from_fn(|_| words.next())
        else {
            tracing::warn!(
                "Could not parse line of /proc/swaps: {line}",
                line = rxfetch::display::DisplayBytes::new(line)
            );
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Faled to parse line {}", DisplayBytes::new(line)),
            ));
        };
        let size: u64 =
            parse_from_bytes(size).map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
        let used: u64 =
            parse_from_bytes(used).map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
        let priority: i32 =
            parse_from_bytes(priority).map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
        // Clamp used to <= size
        let used = used.min(size);
        Ok(Self {
            name: PathBuf::from(OsString::from_vec(name.to_vec())),
            size: size * 1024,
            used: used * 1024,
            priority,
        })
    }
}

impl SwapData {
    pub fn fetch() -> std::io::Result<Self> {
        let swaps = File::open("/proc/swaps")?;
        let mut swaps = BufReader::new(swaps);
        let mut out = Vec::new();
        swaps.for_byte_line(|line| {
            if line.starts_with(b"Filename") {
                return Ok(true);
            }
            let swap = Swap::from_line(line)?;
            out.push(swap);
            Ok(true)
        })?;
        Ok(Self { swaps: out })
    }
}
