use std::{
    fmt::{Debug, Display},
    io::{BufReader, Error, ErrorKind},
};

use crate::util::hex::unhex;
use bstr::io::BufReadExt;

#[derive(Default, Debug)]
pub struct Device {
    pub vid: u16,
    pub did: u16,
    pub name: String,
    pub vendor: String,
    pub subsystems: Vec<Subsystem>,
}
#[derive(Default, Debug)]
pub struct Subsystem {
    pub vid: u16,
    pub did: u16,
    pub name: String,
}

fn open_pci_ids() -> Option<std::fs::File> {
    [
        "/usr/share/hwdata/pci.ids",
        "/usr/share/misc/pci.ids",
        "/usr/share/pci.ids",
    ]
    .iter()
    .find_map(|path| std::fs::File::open(path).ok())
}
const fn u16_to_hex(value: u16) -> [u8; 4] {
    const DIGITS: [u8; 16] = *b"0123456789abcdef";
    let mut buf = [0; 4];
    let mut i = 0;
    while i < buf.len() {
        let digit = (value >> (12 - i * 4)) as usize % DIGITS.len();
        buf[i] = DIGITS[digit];
        i += 1;
    }
    buf
}
fn hex_to_u16(data: &[u8; 4]) -> std::io::Result<u16> {
    if !data.iter().all(u8::is_ascii_hexdigit) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Failed to parse subvendor ID",
        ));
    }
    Ok(data
        .chunks_exact(2)
        .map(|bytes| (unhex(bytes[0]) << 4) + unhex(bytes[1]))
        .fold(0, |acc, hex| (acc << 8) | hex as u16))
}
#[derive(Debug)]
enum FindStage {
    None,
    FoundVendor(String),
    FoundDevice(Device),
}
pub fn read_pci_device(vid: u16, did: u16) -> std::io::Result<Option<Device>> {
    let pci_db = open_pci_ids()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "PCI ID Database not found."))?;
    let mut pci_db = BufReader::new(pci_db);
    // Pass 1: Find device entry
    let hex_vendor = u16_to_hex(vid);
    let hex_device = u16_to_hex(did);
    let mut stage = FindStage::None;
    pci_db.for_byte_line(|line| {
        // Remove comments
        let line = line.split(|&b| b == b'#').next().unwrap_or_default();
        if line.is_empty() {
            return Ok(true);
        }
        let tabs: &'static [u8] = match stage {
            FindStage::None => b"",
            FindStage::FoundVendor(..) => b"\t",
            FindStage::FoundDevice(..) => b"\t\t",
        };
        let Some(line) = line.strip_prefix(tabs) else {
            return Ok(false);
        };
        stage = match &mut stage {
            FindStage::None => {
                let Some(rest) = line.strip_prefix(&hex_vendor) else {
                    return Ok(true);
                };
                let vendor = rest.trim_ascii();
                let vendor = match core::str::from_utf8(vendor) {
                    Ok(name) => name,
                    Err(err) => return Err(Error::new(ErrorKind::InvalidData, err)),
                };
                FindStage::FoundVendor(vendor.to_string())
            }
            FindStage::FoundVendor(vendor) => {
                let Some(rest) = line.strip_prefix(&hex_device) else {
                    return Ok(true);
                };
                let name = rest.trim_ascii();
                let name = match core::str::from_utf8(name) {
                    Ok(name) => name,
                    Err(err) => return Err(Error::new(ErrorKind::InvalidData, err)),
                };
                FindStage::FoundDevice(Device {
                    vid,
                    did,
                    name: name.to_string(),
                    vendor: core::mem::take(vendor),
                    subsystems: Vec::new(),
                })
            }
            FindStage::FoundDevice(dev) => {
                let Some((hex_subvendor, rest)) = line.split_at_checked(4) else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Failed to parse subvendor ID",
                    ));
                };
                let rest = rest.trim_ascii();
                let subvendor_id = hex_to_u16(hex_subvendor.try_into().unwrap())?;
                let Some((hex_subdevice, rest)) = rest.split_at_checked(4) else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Failed to parse subdevice ID",
                    ));
                };
                let subdevice_id = hex_to_u16(hex_subdevice.try_into().unwrap())?;
                let name = match core::str::from_utf8(rest.trim_ascii()) {
                    Ok(name) => name,
                    Err(err) => return Err(Error::new(ErrorKind::InvalidData, err)),
                };
                dev.subsystems.push(Subsystem {
                    vid: subvendor_id,
                    did: subdevice_id,
                    name: name.to_string(),
                });
                FindStage::FoundDevice(core::mem::take(dev))
            }
        };
        Ok(true)
    })?;
    match stage {
        FindStage::None | FindStage::FoundVendor(..) => Ok(None),
        FindStage::FoundDevice(dev) => Ok(Some(dev)),
    }
}

pub struct PrettyDevice<'dev>(pub &'dev Device);
impl Display for PrettyDevice<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[inline(always)]
        fn get_pretty_name(long: &str) -> &str {
            let (Some(start), Some(end)) = (long.find('['), long.find(']')) else {
                return long;
            };
            &long[start + 1..end]
        }

        let card = self.0;

        let vendor = &card.vendor;
        let vendor = get_pretty_name(vendor);

        let mut name = &card.name;

        if let Some(sub) = card.subsystems.iter().find(|s| s.name.contains('[')) {
            name = &sub.name;
        }

        let name = get_pretty_name(name);

        // Shorten GPU text
        let (name, suffix) = name
            .find(" Laptop GPU")
            .map(|end| (&name[..end], "(Laptop)"))
            .or_else(|| name.find(" Integrated").map(|end| (&name[..end], " iGPU")))
            .unwrap_or((name, ""));
        let name = name.strip_prefix("Sapphire Pulse").unwrap_or(name);

        // Shorten vendor
        let vendor = vendor
            .find(' ')
            .map(|end| &vendor[..end])
            .and_then(|firstword| {
                firstword
                    .bytes()
                    .next()
                    .is_some_and(|b| b.is_ascii_uppercase())
                    .then_some(firstword)
            })
            .unwrap_or(vendor.trim());
        // Remove alternative names
        let vendor = vendor.split('/').next().unwrap_or(vendor);

        // Remove whitespace
        let name = name.trim();

        write!(f, "{vendor} {name}{suffix}")
    }
}
impl Debug for PrettyDevice<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let card = self.0;

        let vendor = &card.vendor;
        let name = &card.name;

        // Remove whitespace
        let name = name.trim();

        write!(f, "{vendor} {name}")
    }
}
