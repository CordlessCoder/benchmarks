use nix::{fcntl::OFlag, sys::stat::Mode};
use std::{fmt::Debug, fs, io::Read};
#[cfg(feature = "tracing")]
use tracing::warn;

use crate::{
    cached_path::CachedPath,
    parse::{hex, unhex},
    ArrayVec,
};

use super::{NoProvider, PciBackendError, PciDevIterBackend, PciDevice, PciInfoProvider, WrapPath};

#[derive(Debug)]
pub struct SysBusBackend {
    dir_iter: nix::dir::OwningIter,
}

#[derive(Debug, Clone)]
pub struct SysBusProvider;

impl SysBusProvider {
    #[must_use]
    pub fn path_for_device(
        &self,
        PciDevice {
            domain,
            bus,
            device,
            function,
            ..
        }: &PciDevice<Self>,
    ) -> CachedPath {
        use core::fmt::Write;
        let mut path = CachedPath::take();
        let storage = path.as_mut_os_string();
        storage.clear();
        storage.push("/sys/bus/pci/devices/");
        _ = write!(
            storage,
            "{domain:0>4x}:{bus:0>2x}:{device:0>2x}.{function:0>1x}"
        );
        path
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
fn parse_device(input: &[u8]) -> Result<PciDevice<NoProvider>, nom::Err<(), ()>> {
    use nom::bytes::tag;
    use nom::Parser;
    let mut parser = (hex(4), tag(":"), hex(2), tag(":"), hex(2), tag("."), hex(1)).map(
        |(domain, _, bus, _, device, _, function)| PciDevice {
            domain: domain as _,
            bus: bus as _,
            device: device as _,
            function: function as _,
            ..PciDevice::new(0, 0, 0, 0)
        },
    );
    parser.parse_complete(input).map(|(_, d)| d)
}

impl PciInfoProvider for SysBusProvider {
    fn get_class(dev: &mut PciDevice<Self>) -> Result<ArrayVec<u8, 32>, PciBackendError> {
        let mut path = dev.provider.path_for_device(dev);
        // Temporarily add "class" to the end of the path, removing it once WrapPath goes out of
        // scope
        let class = WrapPath::new(&mut path, "class");

        let mut file = fs::File::open(&*class).map_err(PciBackendError::IOError)?;
        let mut buf: ArrayVec<u8, 64> = ArrayVec::new();
        std::io::copy(&mut file, &mut buf).map_err(PciBackendError::IOError)?;
        Ok(buf
            .chunks_exact(2)
            // Skip leading 0x
            .skip(1)
            .map(|bytes| (unhex(bytes[0]) << 4) + unhex(bytes[1]))
            .collect())
    }
    fn get_vendor(dev: &mut PciDevice<Self>) -> Result<u16, PciBackendError> {
        let mut path = dev.provider.path_for_device(dev);
        let vendor = WrapPath::new(&mut path, "vendor");
        let vendor = vendor.as_path();

        let mut file = fs::File::open(vendor).map_err(PciBackendError::IOError)?;

        let mut buf: ArrayVec<u8, 32> = ArrayVec::new();
        std::io::copy(&mut file, &mut buf).map_err(PciBackendError::IOError)?;

        // Skip leading 0x
        let bytes = buf.get(2..6).ok_or(PciBackendError::InvalidDevice)?;
        Ok(bytes
            .chunks_exact(2)
            .map(|bytes| (unhex(bytes[0]) << 4) + unhex(bytes[1]))
            .fold(0, |acc, hex| (acc << 8) | hex as u16))
    }

    fn get_device(dev: &mut PciDevice<Self>) -> Result<u16, PciBackendError> {
        let mut path = dev.provider.path_for_device(dev);
        let device = WrapPath::new(&mut path, "device");
        let device = device.as_path();

        let mut file = fs::File::open(device).map_err(PciBackendError::IOError)?;

        let mut buf = [0; 32];
        file.read(&mut buf).map_err(PciBackendError::IOError)?;

        // Skip leading 0x
        let bytes = buf.get(2..6).ok_or(PciBackendError::InvalidDevice)?;
        Ok(bytes
            .chunks_exact(2)
            .map(|bytes| (unhex(bytes[0]) << 4) | unhex(bytes[1]))
            .fold(0, |acc, hex| (acc << 8) | hex as u16))
    }
    fn get_susbystem_vid(dev: &mut PciDevice<Self>) -> Result<u16, PciBackendError> {
        let mut path = dev.provider.path_for_device(dev);
        let path = WrapPath::new(&mut path, "subsystem_vendor");
        let path = path.as_path();

        let mut file = fs::File::open(path).map_err(PciBackendError::IOError)?;

        let mut buf: ArrayVec<u8, 32> = ArrayVec::new();
        std::io::copy(&mut file, &mut buf).map_err(PciBackendError::IOError)?;

        // Skip leading 0x
        let bytes = buf.get(2..6).ok_or(PciBackendError::InvalidDevice)?;
        Ok(bytes
            .chunks_exact(2)
            .map(|bytes| (unhex(bytes[0]) << 4) + unhex(bytes[1]))
            .fold(0, |acc, hex| (acc << 8) | hex as u16))
    }
    fn get_susbystem_did(dev: &mut PciDevice<Self>) -> Result<u16, PciBackendError> {
        let mut path = dev.provider.path_for_device(dev);
        let path = WrapPath::new(&mut path, "subsystem_device");
        let path = path.as_path();

        let mut file = fs::File::open(path).map_err(PciBackendError::IOError)?;

        let mut buf: ArrayVec<u8, 32> = ArrayVec::new();
        std::io::copy(&mut file, &mut buf).map_err(PciBackendError::IOError)?;

        // Skip leading 0x
        let bytes = buf.get(2..6).ok_or(PciBackendError::InvalidDevice)?;
        Ok(bytes
            .chunks_exact(2)
            .map(|bytes| (unhex(bytes[0]) << 4) + unhex(bytes[1]))
            .fold(0, |acc, hex| (acc << 8) | hex as u16))
    }

    fn get_revision(dev: &mut PciDevice<Self>) -> Result<u8, PciBackendError> {
        let mut path = dev.provider.path_for_device(dev);
        let path = WrapPath::new(&mut path, "revision");
        let path = path.as_path();

        let mut file = fs::File::open(path).map_err(PciBackendError::IOError)?;

        let mut buf: ArrayVec<u8, 32> = ArrayVec::new();
        std::io::copy(&mut file, &mut buf).map_err(PciBackendError::IOError)?;

        let bytes = buf.get(2..4).ok_or(PciBackendError::InvalidDevice)?;
        Ok((unhex(bytes[0]) << 4) | unhex(bytes[1]))
    }
}

impl PciDevIterBackend for SysBusBackend {
    type BackendInfoProvider = SysBusProvider;

    fn try_init() -> Result<Self, PciBackendError> {
        let dir = nix::dir::Dir::open(
            "/sys/bus/pci/devices",
            OFlag::O_RDONLY | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| PciBackendError::NotAvailable)?;
        let dir_iter = dir.into_iter();
        Ok(Self { dir_iter })
    }
}
impl Iterator for SysBusBackend {
    type Item = Result<PciDevice<SysBusProvider>, PciBackendError>;

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let dir = self.dir_iter.by_ref().next()?;
            let dir = match dir {
                Ok(dir) => dir,
                Err(err) => return Some(Err(PciBackendError::IOError(err.into()))),
            };
            let name = dir.file_name().to_bytes();
            if name == b"." || name == b".." {
                continue;
            }

            let dev = match parse_device(name) {
                Ok(dev) => dev,
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    warn!(
                        "Failed to parse PCI device: `{name}` Error: {_err:?}",
                        name = String::from_utf8_lossy(name)
                    );
                    break Some(Err(PciBackendError::InvalidDevice));
                }
            };

            break Some(Ok(dev.with_provider(SysBusProvider)));
        }
    }
}
