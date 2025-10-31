use pnet::datalink::NetworkInterface;

#[derive(Debug, Clone)]
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
