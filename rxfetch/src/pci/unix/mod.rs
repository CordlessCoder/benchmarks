mod linux_sysfs;
pub use linux_sysfs::*;
mod linux_procfs;
pub use linux_procfs::*;
// mod flags;
use super::{NoProvider, PciBackendError, PciDevIterBackend, PciDevice, PciInfoProvider, WrapPath};
// pub use flags::PciResourceFlags;

// TODO: Add support for PCI resources, to eventually get available vram
// decode flags according to https://elixir.bootlin.com/linux/latest/source/include/linux/ioport.h
// pub struct PciDeviceResource {
//     addr: usize,
//     len: usize,
//     flags: PciResourceFlags,
// }
//
// impl PciDeviceResource {
//     pub fn new(addr: usize, len: usize, flags: u64) -> Self {
//         let flags = PciResourceFlags::from_bits_retain(flags);
//         Self { flags, addr, len }
//     }
// }
