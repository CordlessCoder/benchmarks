use std::{
    collections::HashMap,
    io::{BufReader, Error, ErrorKind},
};
use bstr::io::BufReadExt;
use crate::util::hex::hex_to_u16;

fn open_usb_ids() -> Option<std::fs::File> {
    [
        "/usr/share/hwdata/usb.ids",
        "/usr/share/misc/usb.ids",
        "/usr/share/usb.ids",
        "/var/lib/usbutils/usb.ids",
    ]
    .iter()
    .find_map(|path| std::fs::File::open(path).ok())
}

#[derive(Default, Debug, Clone)]
pub struct NamedUsbDevice {
    pub vid: u16,
    pub did: u16,
    pub name: String,
    pub vendor: String,
}

#[derive(Debug)]
enum FindStage {
    None,
    FoundVendor(u16, String),
}

pub fn query_usb_devices(
    queries: impl IntoIterator<Item = (u16, u16)>,
) -> std::io::Result<Vec<NamedUsbDevice>> {
    let mut vendor_to_devices: HashMap<u16, Vec<u16>> = HashMap::new();
    for (vid, did) in queries {
        vendor_to_devices.entry(vid).or_default().push(did);
    }
    let target_device_count = vendor_to_devices.values().flatten().count();
    let mut found_devices = Vec::new();
    let usb_db = open_usb_ids()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "PCI ID Database not found."))?;
    let mut usb_db = BufReader::new(usb_db);
    let mut stage = FindStage::None;
    usb_db.for_byte_line(|line| {
        if found_devices.len() == target_device_count {
            // Early return once we found all the devices we wanted
            return Ok(false);
        }
        // Remove comments
        let mut line = line.split(|&b| b == b'#').next().unwrap_or_default();
        if line.is_empty() {
            return Ok(true);
        }
        let tabs = line.iter().take_while(|&&b| b == b'\t').count();
        match &mut stage {
            FindStage::None => (),
            FindStage::FoundVendor(..) if tabs == 1 => {
                line = &line[1..];
            }
            FindStage::FoundVendor(..) => stage = FindStage::None,
        };
        match &mut stage {
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
                stage = FindStage::FoundVendor(vendor_id, vendor_name.to_string())
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
                };
                let name = line[4..].trim_ascii();
                let name = match core::str::from_utf8(name) {
                    Ok(name) => name,
                    Err(err) => return Err(Error::new(ErrorKind::InvalidData, err)),
                };
                found_devices.push(NamedUsbDevice {
                    name: name.to_string(),
                    did: device_id,
                    vid: *vendor_id,
                    vendor: vendor_name.clone(),
                });
            }
        };
        Ok(true)
    })?;
    Ok(found_devices)
}
