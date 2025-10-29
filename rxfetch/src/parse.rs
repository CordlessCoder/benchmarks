use nom::Parser;

#[must_use]
pub fn hex<I, E>(len: usize) -> impl nom::Parser<I, Output = u32, Error = E>
where
    E: nom::error::ParseError<I>,
    <I as nom::Input>::Item: nom::AsChar,
    I: nom::AsBytes + nom::Input,
{
    nom::bytes::take(len).and_then(nom::number::complete::hex_u32)
}

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
                _ => 0,
            };
            i += 1;
        }
        arr
    };
    LUT[b as usize]
}
