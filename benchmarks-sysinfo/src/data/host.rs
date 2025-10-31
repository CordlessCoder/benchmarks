use rxfetch::{components::name::SystemName, display::DisplayBytes};

#[derive(Debug, Clone)]
pub struct HostData {
    pub hostname: String,
    pub kernel: String,
}

impl HostData {
    pub fn fetch() -> std::io::Result<Self> {
        let systemname = SystemName::get()?;
        let kernel = {
            let text = systemname.release();
            let text = text.0.split(|&b| b == b'-').next().unwrap_or_default();
            DisplayBytes::new(text).to_string()
        };
        Ok(HostData {
            hostname: systemname.node().to_string(),
            kernel,
        })
    }
}
