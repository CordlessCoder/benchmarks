use owo_colors::Style;
use std::{borrow::Cow, fmt::Debug};

pub mod cpu;
pub mod gpu;
pub mod host;
pub mod ip;
pub mod mem;
pub mod pci_totals;
pub mod swap;
pub mod user;

pub struct StyledText {
    pub style: Style,
    pub text: Cow<'static, str>,
}

pub struct DataRow {
    pub label: Cow<'static, str>,
    pub values: Vec<StyledText>,
}

impl DataRow {
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        DataRow {
            label: label.into(),
            values: Vec::new(),
        }
    }
    pub fn push_value(&mut self, text: impl Into<Cow<'static, str>>, style: Style) {
        self.values.push(StyledText {
            text: text.into(),
            style,
        });
    }
    pub fn with_value(mut self, text: impl Into<Cow<'static, str>>, style: Style) -> Self {
        self.push_value(text, style);
        self
    }
}

pub trait DataProvider: Debug + Sync {
    fn identifier(&self) -> &'static str;
    fn try_fetch(&self) -> Result<Vec<DataRow>, String>;
}
