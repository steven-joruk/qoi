use std::{error::Error, fmt::Display};

mod decode;
pub use decode::QoiDecode;

mod encode;
pub use encode::QoiEncode;

#[derive(Debug)]
pub enum QoiError {
    InputSmallerThanHeader,
    IncorrectHeaderMagic,
    Channels,
    InputSize,
    OutputTooSmall,
    InvalidHeader,
    TooBig,
    Io(std::io::Error),
}

impl Error for QoiError {}

impl Display for QoiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputSmallerThanHeader => {
                f.write_str("The input is too small to contain a header")
            }
            Self::IncorrectHeaderMagic => f.write_str("The header magic value i wrong"),
            Self::Channels => f.write_str("The number of channels is invalid"),
            Self::InputSize => f.write_str("The input size is invalid"),
            Self::OutputTooSmall => f.write_str("The output buffer is too small"),
            Self::InvalidHeader => f.write_str("The header is invalid"),
            Self::TooBig => f.write_str("The image size is too big"),
            Self::Io(inner) => {
                f.write_fmt(format_args!("An I/O error occurred: {}", inner.to_string()))
            }
        }
    }
}

impl From<std::io::Error> for QoiError {
    fn from(error: std::io::Error) -> Self {
        QoiError::Io(error)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Channels {
    Three,
    Four,
}

impl TryFrom<u8> for Channels {
    type Error = QoiError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let channels = match value {
            3 => Self::Three,
            4 => Self::Four,
            _ => return Err(QoiError::Channels),
        };
        Ok(channels)
    }
}

impl Channels {
    #[inline]
    fn len(&self) -> u8 {
        match self {
            Self::Three => 3,
            Self::Four => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Default for Pixel {
    #[inline]
    fn default() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
}

impl Pixel {
    #[inline]
    fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    fn cache_index(&self) -> usize {
        (self.r ^ self.g ^ self.b ^ self.a) as usize % 64
    }

    #[inline]
    fn modify_r(&mut self, change: i8) {
        self.r = self.r.wrapping_add(change as u8);
    }

    #[inline]
    fn modify_g(&mut self, change: i8) {
        self.g = self.g.wrapping_add(change as u8);
    }

    #[inline]
    fn modify_b(&mut self, change: i8) {
        self.b = self.b.wrapping_add(change as u8);
    }

    #[inline]
    fn modify_a(&mut self, change: i8) {
        self.a = self.a.wrapping_add(change as u8);
    }
}

pub struct Qoi;

impl Qoi {
    const HEADER_SIZE: usize = 14;
    const PADDING: u8 = 4;
    const MAX_SIZE: usize = 1024 * 1024 * 1024;

    const INDEX: u8 = 0;

    const RUN_8: u8 = 0b0100_0000;
    const RUN_16: u8 = 0b0110_0000;
    const DIFF_8: u8 = 0b1000_0000;
    const DIFF_16: u8 = 0b1100_0000;
    const DIFF_24: u8 = 0b1110_0000;
    const COLOR: u8 = 0b1111_0000;

    const MASK_2: u8 = 0b1100_0000;
    const MASK_3: u8 = 0b1110_0000;
    const MASK_4: u8 = 0b1111_0000;
}

#[derive(Debug)]
pub struct QoiHeader {
    width: u32,
    height: u32,
    channels: Channels,
    colour_space: u8,
}

impl QoiHeader {
    pub fn new(width: u32, height: u32, channels: Channels, colour_space: u8) -> Self {
        Self {
            width,
            height,
            channels,
            colour_space,
        }
    }

    fn to_array(&self) -> [u8; Qoi::HEADER_SIZE] {
        let mut dest = [0u8; Qoi::HEADER_SIZE];

        dest[0..4].copy_from_slice(b"qoif");
        dest[4..8].copy_from_slice(&self.width.to_be_bytes());
        dest[8..12].copy_from_slice(&self.height.to_be_bytes());
        dest[12] = self.channels.len();
        dest[13] = self.colour_space;

        dest
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The size of the image in its raw, uncompressed format.
    pub fn raw_image_size(&self, channels: Channels) -> usize {
        let width = self.width as usize;
        let height = self.height as usize;
        let channels = channels.len() as usize;
        width.saturating_mul(height).saturating_mul(channels)
    }

    pub fn channels(&self) -> Channels {
        self.channels
    }

    pub fn colour_space(&self) -> u8 {
        self.colour_space
    }

    fn new_from_slice(input: &[u8]) -> Result<Self, QoiError> {
        if input.len() < Qoi::HEADER_SIZE as usize {
            return Err(QoiError::InputSmallerThanHeader);
        }

        if &input[0..4] != b"qoif" {
            return Err(QoiError::IncorrectHeaderMagic);
        }

        let header = QoiHeader {
            width: u32::from_be_bytes(input[4..8].try_into().unwrap()),
            height: u32::from_be_bytes(input[8..12].try_into().unwrap()),
            channels: input[12].try_into()?,
            colour_space: input[13],
        };

        Ok(header)
    }
}

trait IsBetween: PartialOrd
where
    Self: Sized,
{
    #[inline]
    fn is_between(&self, low: Self, high: Self) -> bool {
        *self >= low && *self <= high
    }
}

impl IsBetween for i16 {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pixel_diff_wraps() {
        let mut pixel = Pixel::new(10, 10, 10, 10);
        pixel.modify_r(-13);
        pixel.modify_g(-13);
        pixel.modify_b(-13);
        pixel.modify_a(-13);
        assert_eq!(pixel, Pixel::new(253, 253, 253, 253));

        let mut pixel = Pixel::new(250, 250, 250, 250);
        pixel.modify_r(7);
        pixel.modify_g(7);
        pixel.modify_b(7);
        pixel.modify_a(7);
        assert_eq!(pixel, Pixel::new(1, 1, 1, 1));
    }
}
