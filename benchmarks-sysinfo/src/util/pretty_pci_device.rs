use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    io::{BufReader, Error, ErrorKind},
};

use crate::util::hex::unhex;
use bstr::io::BufReadExt;

#[derive(Default, Debug, Clone)]
pub struct Device {
    pub vid: u16,
    pub did: u16,
    pub name: String,
    pub vendor: String,
    pub subsystems: Vec<Subsystem>,
}
#[derive(Default, Debug, Clone)]
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
fn hex_to_u16(data: &[u8; 4]) -> Option<u16> {
    if !data.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    Some(
        data.chunks_exact(2)
            .map(|bytes| (unhex(bytes[0]) << 4) + unhex(bytes[1]))
            .fold(0, |acc, hex| (acc << 8) | hex as u16),
    )
}

fn hex_to_u16_ioerr(data: &[u8; 4]) -> std::io::Result<u16> {
    hex_to_u16(data)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Failed to parse subvendor ID"))
}
#[derive(Debug)]
enum FindStage {
    None,
    FoundVendor(u16, String),
    FoundDevice(Device),
}

// TODO: Add support for simultaneously fetching PCI IDs of multiple vendor-device pairs.
pub fn query_pci_devices(
    queries: impl IntoIterator<Item = (u16, u16)>,
) -> std::io::Result<Vec<Device>> {
    let mut vendor_to_devices: HashMap<u16, Vec<u16>> = HashMap::new();
    for (vid, did) in queries {
        // let hex_vendor = u16_to_hex(vid);
        // let hex_device = u16_to_hex(did);
        vendor_to_devices.entry(vid).or_default().push(did);
    }
    let mut found_devices = Vec::new();
    let pci_db = open_pci_ids()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "PCI ID Database not found."))?;
    let mut pci_db = BufReader::new(pci_db);
    let mut stage = FindStage::None;
    pci_db.for_byte_line(|line| {
        // Remove comments
        let mut line = line.split(|&b| b == b'#').next().unwrap_or_default();
        if line.is_empty() {
            return Ok(true);
        }
        match &mut stage {
            FindStage::None => (),
            FindStage::FoundVendor(..) if line.starts_with(b"\t") => {
                line = &line[1..];
            }
            FindStage::FoundDevice(..) if line.starts_with(b"\t\t") => {
                line = &line[2..];
            }
            FindStage::FoundDevice(dev) if line.starts_with(b"\t") => {
                let replacemement = FindStage::FoundVendor(dev.vid, dev.vendor.clone());
                let device_stage = core::mem::replace(&mut stage, replacemement);
                let FindStage::FoundDevice(dev) = device_stage else {
                    unreachable!()
                };
                found_devices.push(dev);
                line = &line[1..];
            }
            FindStage::FoundDevice(_) => {
                let device_stage = core::mem::replace(&mut stage, FindStage::None);
                let FindStage::FoundDevice(dev) = device_stage else {
                    unreachable!()
                };
                found_devices.push(dev);
            }
            FindStage::FoundVendor(..) => stage = FindStage::None,
        };
        stage = match &mut stage {
            FindStage::None => {
                let Some(vendor_id) = line
                    .get(..4)
                    .and_then(|hex| hex_to_u16(hex.try_into().unwrap()))
                else {
                    return Ok(true);
                };
                if !vendor_to_devices.contains_key(&vendor_id) {
                    return Ok(true);
                }
                let vendor = line[4..].trim_ascii();
                let vendor_name = match core::str::from_utf8(vendor) {
                    Ok(name) => name,
                    Err(err) => return Err(Error::new(ErrorKind::InvalidData, err)),
                };
                FindStage::FoundVendor(vendor_id, vendor_name.to_string())
            }
            FindStage::FoundVendor(vendor_id, vendor_name) => {
                let Some(device_id) = line
                    .get(..4)
                    .and_then(|hex| hex_to_u16(hex.try_into().unwrap()))
                else {
                    return Ok(true);
                };
                if !vendor_to_devices
                    .get(vendor_id)
                    .unwrap()
                    .contains(&device_id)
                {
                    return Ok(true);
                }
                let name = line[4..].trim_ascii();
                let name = match core::str::from_utf8(name) {
                    Ok(name) => name,
                    Err(err) => return Err(Error::new(ErrorKind::InvalidData, err)),
                };
                FindStage::FoundDevice(Device {
                    vid: *vendor_id,
                    did: device_id,
                    name: name.to_string(),
                    vendor: vendor_name.clone(),
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
                let subvendor_id = hex_to_u16_ioerr(hex_subvendor.try_into().unwrap())?;
                let Some((hex_subdevice, rest)) = rest.split_at_checked(4) else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Failed to parse subdevice ID",
                    ));
                };
                let subdevice_id = hex_to_u16_ioerr(hex_subdevice.try_into().unwrap())?;
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
    if let FindStage::FoundDevice(dev) = stage {
        found_devices.push(dev);
    }
    Ok(found_devices)
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
