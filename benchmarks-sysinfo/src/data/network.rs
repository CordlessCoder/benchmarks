use pnet::datalink::NetworkInterface;

pub struct NetworkData {
    pub interfaces: Vec<NetworkInterface>,
}

impl NetworkData {
    pub fn fetch() -> Self {
        Self {
            interfaces: pnet::datalink::interfaces(),
        }
    }
}
