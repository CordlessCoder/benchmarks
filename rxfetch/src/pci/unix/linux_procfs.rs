use super::{NoProvider, PciBackendError, PciDevIterBackend, PciDevice, PciInfoProvider};
use crate::{cached_path::CachedPath, parse::hex, ArrayVec};
use nix::{fcntl::OFlag, sys::stat::Mode};
use nom::Parser;
use std::{ffi::OsStr, fmt::Debug, os::unix::ffi::OsStrExt, path::Path};

const PROC_BUS_PCI: &str = "/proc/bus/pci/";

// bus/device.function
#[derive(Debug)]
pub struct ProcBusBackend {
    bus_iter: nix::dir::OwningIter,
    bus: Option<(nix::dir::OwningIter, u8)>,
}

#[derive(Debug, Clone)]
pub struct ProcBusProvider {
    buf: ArrayVec<u8, 72>,
}

impl ProcBusProvider {
    pub fn from_devfile<P: AsRef<Path>>(file: P) -> Result<Self, PciBackendError> {
        let path = file.as_ref();
        let mut file = std::fs::File::open(path).map_err(PciBackendError::IOError)?;
        let mut buf = ArrayVec::new();
        std::io::copy(&mut file, &mut buf).map_err(PciBackendError::IOError)?;
        if buf.len() < 16 {
            return Err(PciBackendError::InvalidDevice);
        }
        Ok(ProcBusProvider { buf })
    }
}

fn parse_device(input: &[u8]) -> Result<PciDevice<NoProvider>, nom::Err<(), ()>> {
    use nom::bytes::tag;
    use nom::Parser;
    let mut parser = (hex(2), tag("."), hex(1)).map(|(device, _, function)| PciDevice {
        device: device as _,
        function: function as _,
        ..PciDevice::new(0, 0, 0, 0)
    });
    parser.parse_complete(input).map(|(_, d)| d)
}

#[must_use]
fn get_header_type(dev: &PciDevice<ProcBusProvider>) -> u8 {
    dev.provider.buf[14]
}

impl PciInfoProvider for ProcBusProvider {
    fn get_class(dev: &mut PciDevice<Self>) -> Result<ArrayVec<u8, 32>, PciBackendError> {
        let class = dev.provider.buf[11];
        let subclass = dev.provider.buf[10];
        Ok(ArrayVec::from_iter([class, subclass]))
    }
    fn get_vendor(dev: &mut PciDevice<Self>) -> Result<u16, PciBackendError> {
        Ok(u16::from_le_bytes(
            dev.provider.buf[0..2].try_into().unwrap(),
        ))
    }
    fn get_device(dev: &mut PciDevice<Self>) -> Result<u16, PciBackendError> {
        Ok(u16::from_le_bytes(
            dev.provider.buf[2..4].try_into().unwrap(),
        ))
    }
    fn get_susbystem_vid(dev: &mut PciDevice<Self>) -> Result<u16, PciBackendError> {
        let svid = match get_header_type(dev) {
            0x0 => dev
                .provider
                .buf
                .get(47..49)
                .ok_or(PciBackendError::InvalidDevice)?
                .try_into()
                .unwrap(),
            0x2 => dev
                .provider
                .buf
                .get(66..68)
                .ok_or(PciBackendError::InvalidDevice)?
                .try_into()
                .unwrap(),
            _ => return Err(PciBackendError::NotAvailable),
        };
        Ok(u16::from_le_bytes(svid))
    }
    fn get_susbystem_did(dev: &mut PciDevice<Self>) -> Result<u16, PciBackendError> {
        let sid = match get_header_type(dev) {
            0x0 => dev
                .provider
                .buf
                .get(49..51)
                .ok_or(PciBackendError::InvalidDevice)?
                .try_into()
                .unwrap(),
            0x2 => dev
                .provider
                .buf
                .get(64..66)
                .ok_or(PciBackendError::InvalidDevice)?
                .try_into()
                .unwrap(),
            _ => return Err(PciBackendError::NotAvailable),
        };
        Ok(u16::from_le_bytes(sid))
    }
    fn get_revision(dev: &mut PciDevice<Self>) -> Result<u8, PciBackendError> {
        Ok(dev.provider.buf[8])
    }
}

impl PciDevIterBackend for ProcBusBackend {
    type BackendInfoProvider = ProcBusProvider;

    fn try_init() -> Result<Self, PciBackendError> {
        let dir = nix::dir::Dir::open(
            "/proc/bus/pci/",
            OFlag::O_RDONLY | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| PciBackendError::NotAvailable)?;
        Ok(Self {
            bus_iter: dir.into_iter(),
            bus: None,
        })
    }
}
impl Iterator for ProcBusBackend {
    type Item = Result<PciDevice<ProcBusProvider>, PciBackendError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Bus currently being iterated over
        loop {
            let Self { bus_iter, bus } = self;
            let Some((devices, bus)) = bus else {
                // Find the next bus to iterate over
                let bus_dir = match bus_iter.next()? {
                    Ok(bus) => bus,
                    Err(err) => return Some(Err(PciBackendError::IOError(err.into()))),
                };
                let name = bus_dir.file_name().to_bytes();
                if name == b"." || name == b".." {
                    continue;
                }
                let mut bus_dir_path = CachedPath::take();
                {
                    let storage = bus_dir_path.as_mut_os_string();
                    storage.clear();
                    storage.push(PROC_BUS_PCI);
                    storage.push(OsStr::from_bytes(name));
                }

                // Ignore devices file
                if name == b"devices" {
                    continue;
                }
                let dev_dir = match nix::dir::Dir::open(
                    &*bus_dir_path,
                    OFlag::O_RDONLY | OFlag::O_CLOEXEC,
                    Mode::empty(),
                ) {
                    Ok(d) => d,
                    Err(err) => return Some(Err(PciBackendError::IOError(err.into()))),
                };
                // Parse bus
                let Ok((_, b)) = hex::<_, ()>(2).parse(name) else {
                    return Some(Err(PciBackendError::InvalidDevice));
                };
                // Store bus
                *bus = Some((dev_dir.into_iter(), b as u8));
                continue;
            };
            let dev = match devices.next() {
                Some(Ok(dev)) => dev,
                Some(Err(err)) => return Some(Err(PciBackendError::IOError(err.into()))),
                None => {
                    // We have exhausted the devices in this bus, remove the bus iterator
                    self.bus = None;
                    continue;
                }
            };
            let filename = dev.file_name().to_bytes();
            if filename == b"." || filename == b".." {
                continue;
            }
            // Attempt to parse the device filename
            let Ok(mut dev) = parse_device(filename) else {
                return Some(Err(PciBackendError::InvalidDevice));
            };
            // Attach the device to the current bus
            dev.bus = *bus;
            let mut dev_path = CachedPath::take();
            {
                use core::fmt::Write;
                let storage = dev_path.as_mut_os_string();
                storage.clear();
                storage.push(PROC_BUS_PCI);
                _ = write!(storage, "{bus:0>2x}/");
                storage.push(OsStr::from_bytes(filename));
            }
            let data = match ProcBusProvider::from_devfile(&*dev_path) {
                Ok(p) => p,
                Err(err) => return Some(Err(err)),
            };
            return Some(Ok(dev.with_provider(data)));
        }
    }
}
