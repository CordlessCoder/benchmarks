use std::io;

#[must_use]
pub(crate) const fn unhex(b: u8) -> u8 {
    const LUT: [u8; 256] = {
        let mut arr = [0; 256];
        let mut i = 0;
        while i < 256 {
            let b = i as u8;
            arr[b as usize] = match b {
                b'0'..=b'9' => b.wrapping_sub(b'0'),
                b'a'..=b'f' => b.wrapping_sub(b'a').wrapping_add(10),
                b'A'..=b'F' => b.wrapping_sub(b'A').wrapping_add(10),
                _ => 0,
            };
            i += 1;
        }
        arr
    };
    LUT[b as usize]
}

pub fn hex_to_u16(data: &[u8; 4]) -> Option<u16> {
    if !data.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    Some(
        data.chunks_exact(2)
            .map(|bytes| (unhex(bytes[0]) << 4) + unhex(bytes[1]))
            .fold(0, |acc, hex| (acc << 8) | hex as u16),
    )
}

pub fn hex_to_u16_ioerr(data: &[u8; 4]) -> std::io::Result<u16> {
    hex_to_u16(data)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to parse subvendor ID"))
}
