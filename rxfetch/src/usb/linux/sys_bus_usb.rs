use crate::{cached_path::CachedPath, parse::unhex, usb::UsbDeviceID};
use nix::{dir, fcntl::OFlag, sys::stat::Mode};
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, ErrorKind, Read},
    os::unix::ffi::OsStrExt,
};

const SYS_USB_DEVICES: &str = "/sys/bus/usb/devices/";

pub struct SysBusUsbIter {
    devices: dir::OwningIter,
}

impl SysBusUsbIter {
    pub fn try_init() -> io::Result<Self> {
        let device_dir = nix::dir::Dir::open(
            SYS_USB_DEVICES,
            OFlag::O_RDONLY | OFlag::O_CLOEXEC,
            Mode::empty(),
        )?;
        Ok(Self {
            devices: device_dir.into_iter(),
        })
    }
}

fn read_hex_u16_from_reader(reader: &mut impl Read) -> io::Result<u16> {
    let mut buf = [0; 4];
    reader.read_exact(&mut buf)?;
    if !buf.iter().all(u8::is_ascii_hexdigit) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Found non-hex bytes in hex file",
        ));
    }
    let parsed = buf
        .chunks_exact(2)
        .map(|hex_pair| (unhex(hex_pair[0]) << 4) | (unhex(hex_pair[1])))
        .fold(0, |acc, hex| (acc << 8) | hex as u16);
    Ok(parsed)
}

impl Iterator for SysBusUsbIter {
    type Item = io::Result<UsbDeviceID>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let dev_entry = match self.devices.next()? {
                Ok(dev) => dev,
                Err(err) => return Some(Err(err.into())),
            };
            let dev_folder_name = dev_entry.file_name().to_bytes();
            if matches!(dev_folder_name, b"." | b"..") {
                continue;
            }
            let mut path = CachedPath::take();
            let storage = path.as_mut_os_string();
            storage.clear();
            storage.push(SYS_USB_DEVICES);
            storage.push(OsStr::from_bytes(dev_folder_name));
            path.push("idProduct");
            let mut product_file = match File::open(&*path) {
                Ok(file) => file,
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    // No product/vendor IDs for this device
                    continue;
                }
                Err(err) => return Some(Err(err)),
            };
            let product_id = match read_hex_u16_from_reader(&mut product_file) {
                Ok(id) => id,
                Err(err) => return Some(Err(err)),
            };
            core::mem::drop(product_file);
            path.pop();
            path.push("idVendor");
            let mut vendor_file = match File::open(&*path) {
                Ok(file) => file,
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    // No product/vendor IDs for this device
                    continue;
                }
                Err(err) => return Some(Err(err)),
            };
            let vendor_id = match read_hex_u16_from_reader(&mut vendor_file) {
                Ok(id) => id,
                Err(err) => return Some(Err(err)),
            };
            path.pop();
            path.push("product");
            let mut product = std::fs::read_to_string(&*path).ok();
            if let Some(product) = &mut product {
                if product.ends_with('\n') {
                    product.pop();
                }
            }
            path.pop();
            path.push("manufacturer");
            let mut manufacturer = std::fs::read_to_string(&*path).ok();
            if let Some(manufacturer) = &mut manufacturer {
                if manufacturer.ends_with('\n') {
                    manufacturer.pop();
                }
            }
            return Some(Ok(UsbDeviceID {
                vendor_id,
                product_id,
                product,
                manufacturer,
            }));
        }
    }
}
