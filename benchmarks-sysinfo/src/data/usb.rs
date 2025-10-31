use rxfetch::usb::{UsbDeviceID, sys_bus_usb::SysBusUsbIter};

#[derive(Debug, Clone)]
pub struct UsbData {
    pub device_ids: Vec<UsbDeviceID>,
}

impl UsbData {
    pub fn fetch() -> std::io::Result<Self> {
        let iter = SysBusUsbIter::try_init()?;
        let device_ids: std::io::Result<Vec<UsbDeviceID>> = iter.collect();
        let device_ids = device_ids?;
        // let devices_named = query_usb_devices(device_ids.iter().map(|id| (id.vendor_id, id.product_id)))?;
        Ok(UsbData { device_ids })
    }
}
