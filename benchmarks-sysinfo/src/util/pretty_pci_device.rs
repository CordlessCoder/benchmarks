use std::fmt::{Debug, Display};

#[derive(Default, Debug, Clone)]
pub struct NamedPciDevice {
    pub vid: u16,
    pub did: u16,
    pub name: String,
    pub vendor: String,
    pub subsystems: Vec<Subsystem>,
    pub is_gpu: bool,
}
#[derive(Default, Debug, Clone)]
pub struct Subsystem {
    pub vid: u16,
    pub did: u16,
    pub name: String,
}

pub struct PrettyDevice<'dev>(pub &'dev NamedPciDevice);
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

        let mut name = &*card.name;

        if let Some(sub) = card.subsystems.iter().find(|s| s.name.contains('[')) {
            name = &sub.name;
        }

        if self.0.is_gpu {
            name = get_pretty_name(name);
        }

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
