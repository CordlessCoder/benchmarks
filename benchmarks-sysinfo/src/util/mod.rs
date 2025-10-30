mod pretty_pci_device;
pub use pretty_pci_device::*;
pub mod hex;

use bstr::{ByteSlice, io::BufReadExt};
use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Error, ErrorKind},
    path::Path,
    str::FromStr,
};
use tracing::warn;

pub fn for_colon_separated_line<S>(
    path: impl AsRef<Path>,
    state: &mut S,
    mut cb: impl FnMut(&mut S, &str, &str) -> std::io::Result<bool>,
    mut no_colon_cb: impl FnMut(&mut S, &[u8]) -> std::io::Result<bool>,
) -> std::io::Result<()> {
    let file = File::open(path)?;
    let mut file = BufReader::new(file);
    file.for_byte_line(|line| {
        let Some((name, value)) = line.split_once_str(b":") else {
            return no_colon_cb(state, line);
        };
        let [name, value] = [name, value].map(<[u8]>::trim_ascii);
        let Ok(name) = core::str::from_utf8(name) else {
            warn!(name: "Failed to parse colon separated name", name);
            return Ok(false);
        };
        let Ok(value) = core::str::from_utf8(value) else {
            warn!(name: "Failed to parse colon separated name", value);
            return Ok(false);
        };
        cb(state, name, value)
    })
}

pub fn parse_first_number(value: &str) -> std::io::Result<(u64, &str)> {
    let len = value.bytes().take_while(|b| b.is_ascii_digit()).count();
    let (value, rest) = value.split_at(len);
    let number = value.parse().map_err(|_| {
        Error::new(
            ErrorKind::InvalidData,
            "Failed to parse number in input value",
        )
    })?;
    Ok((number, rest))
}

pub fn parse_from_bytes<T: FromStr>(data: &[u8]) -> Result<T, String>
where
    T::Err: Display,
{
    let Ok(data) = core::str::from_utf8(data) else {
        return Err(String::from("Invalid UTF-8"));
    };
    data.parse().map_err(|err: T::Err| err.to_string())
}
