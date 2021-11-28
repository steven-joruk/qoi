use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum QoiError {
    InputSmallerThanHeader,
    IncorrectHeaderMagic,
    Channels,
    InputSize,
    OutputTooSmall,
    InvalidHeader,
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
        self.width() as usize * self.height() as usize * channels.len() as usize
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

pub trait QoiEncode {
    fn qoi_encode(
        &self,
        width: u32,
        height: u32,
        channels: Channels,
        colour_space: u8,
        dest: impl AsMut<[u8]>,
    ) -> Result<usize, QoiError>;

    fn qoi_encode_to_vec(
        &self,
        width: u32,
        height: u32,
        channels: Channels,
        colour_space: u8,
    ) -> Result<Vec<u8>, QoiError>;
}

impl<S> QoiEncode for S
where
    S: AsRef<[u8]>,
{
    fn qoi_encode(
        &self,
        width: u32,
        height: u32,
        channels: Channels,
        colour_space: u8,
        mut dest: impl AsMut<[u8]>,
    ) -> Result<usize, QoiError> {
        let dest = dest.as_mut();

        let src = self.as_ref();
        let mut cache = [Pixel::default(); 64];
        let mut previous_pixel = Pixel::new(0, 0, 0, 255);
        let mut run = 0u16;
        let header = QoiHeader::new(width, height, channels, colour_space);

        let raw_image_size = header.raw_image_size(channels);
        if src.len() < raw_image_size {
            return Err(QoiError::InputSize);
        }

        dest[0..Qoi::HEADER_SIZE].copy_from_slice(&header.to_array());
        let mut dest_pos = Qoi::HEADER_SIZE;

        for src_pos in (0..raw_image_size).step_by(channels.len() as usize) {
            let a = if channels.len() == 4 {
                src[src_pos + 3]
            } else {
                255
            };

            let pixel = Pixel::new(src[src_pos], src[src_pos + 1], src[src_pos + 2], a);

            if pixel == previous_pixel {
                run += 1;
            }

            if run > 0
                && (pixel != previous_pixel
                    || run == 0x2020
                    || src_pos == (raw_image_size - channels.len() as usize))
            {
                if run < 33 {
                    run -= 1;
                    dest[dest_pos] = Qoi::RUN_8 | (run as u8);
                    dest_pos += 1;
                } else {
                    run -= 33;
                    dest[dest_pos] = Qoi::RUN_16 | ((run >> 8u16) as u8);
                    dest_pos += 1;

                    dest[dest_pos] = run as u8;
                    dest_pos += 1;
                }

                run = 0;
            }

            if pixel != previous_pixel {
                let cache_index = pixel.cache_index();

                if pixel == cache[cache_index] {
                    dest[dest_pos] = Qoi::INDEX | (cache_index as u8);
                    dest_pos += 1;
                } else {
                    cache[cache_index] = pixel;

                    let dr = pixel.r as i16 - previous_pixel.r as i16;
                    let dg = pixel.g as i16 - previous_pixel.g as i16;
                    let db = pixel.b as i16 - previous_pixel.b as i16;
                    let da = pixel.a as i16 - previous_pixel.a as i16;

                    #[inline]
                    fn can_diff_8(dr: i16, dg: i16, db: i16, da: i16) -> bool {
                        da == 0
                            && dr.is_between(-2, 1)
                            && dg.is_between(-2, 1)
                            && db.is_between(-2, 1)
                    }

                    #[inline]
                    fn diff_8(dr: i16, dg: i16, db: i16) -> u8 {
                        Qoi::DIFF_8 | ((dr + 2) << 4) as u8 | ((dg + 2) << 2) as u8 | (db + 2) as u8
                    }

                    #[inline]
                    fn can_diff_16(dr: i16, dg: i16, db: i16, da: i16) -> bool {
                        da == 0
                            && dr.is_between(-16, 15)
                            && dg.is_between(-8, 7)
                            && db.is_between(-8, 7)
                    }

                    #[inline]
                    fn diff_16(dr: i16, dg: i16, db: i16) -> [u8; 2] {
                        let mut dest = [0u8; 2];
                        dest[0] = Qoi::DIFF_16 | (dr + 16) as u8;
                        dest[1] = ((dg + 8) << 4) as u8 | (db + 8) as u8;
                        dest
                    }

                    #[inline]
                    fn can_diff_24(dr: i16, dg: i16, db: i16, da: i16) -> bool {
                        dr.is_between(-16, 15)
                            && dg.is_between(-16, 15)
                            && db.is_between(-16, 15)
                            && da.is_between(-16, 15)
                    }

                    #[inline]
                    fn diff_24(dr: i16, dg: i16, db: i16, da: i16) -> [u8; 3] {
                        let mut dest = [0u8; 3];

                        dest[0] = Qoi::DIFF_24 | ((dr + 16) >> 1) as u8;

                        dest[1] = ((dr + 16) << 7) as u8
                            | ((dg + 16) << 2) as u8
                            | ((db + 16) >> 3) as u8;

                        dest[2] = ((db + 16) << 5) as u8 | (da + 16) as u8;

                        dest
                    }

                    if can_diff_24(dr, dg, db, da) {
                        if can_diff_8(dr, dg, db, da) {
                            dest[dest_pos] = diff_8(dr, dg, db);
                            dest_pos += 1;
                        } else if can_diff_16(dr, dg, db, da) {
                            dest[dest_pos..dest_pos + 2].copy_from_slice(&diff_16(dr, dg, db));
                            dest_pos += 2;
                        } else {
                            dest[dest_pos..dest_pos + 3].copy_from_slice(&diff_24(dr, dg, db, da));
                            dest_pos += 3;
                        }
                    } else {
                        let mut command = Qoi::COLOR;
                        let mut components_written = 0;

                        // The command is written last to avoid extra branches.
                        dest_pos += 1;

                        if dr != 0 {
                            command |= 8;
                            components_written += 1;
                            dest[dest_pos] = pixel.r;
                            dest_pos += 1;
                        }

                        if dg != 0 {
                            command |= 4;
                            components_written += 1;
                            dest[dest_pos] = pixel.g;
                            dest_pos += 1;
                        }

                        if db != 0 {
                            command |= 2;
                            components_written += 1;
                            dest[dest_pos] = pixel.b;
                            dest_pos += 1;
                        }

                        if da != 0 {
                            command |= 1;
                            components_written += 1;
                            dest[dest_pos] = pixel.a;
                            dest_pos += 1;
                        }

                        dest[dest_pos - components_written - 1] = command;
                    }
                }

                previous_pixel = pixel;
            }
        }

        dest[dest_pos..dest_pos + Qoi::PADDING as usize]
            .copy_from_slice(&[0u8; Qoi::PADDING as usize]);
        dest_pos += Qoi::PADDING as usize;

        Ok(dest_pos)
    }

    fn qoi_encode_to_vec(
        &self,
        width: u32,
        height: u32,
        channels: Channels,
        colour_space: u8,
    ) -> Result<Vec<u8>, QoiError> {
        let mut dest = Vec::new();
        dest.resize(
            width as usize * height as usize * channels.len() as usize
                + Qoi::HEADER_SIZE
                + Qoi::PADDING as usize,
            0,
        );
        let size = self.qoi_encode(width, height, channels, colour_space, dest.as_mut_slice())?;
        dest.resize(size, 0);
        Ok(dest)
    }
}

pub trait QoiDecode {
    fn qoi_decode(
        &self,
        channels: Option<Channels>,
        dest: impl AsMut<[u8]>,
    ) -> Result<(), QoiError>;
    fn qoi_decode_to_vec(&self, channels: Option<Channels>) -> Result<Vec<u8>, QoiError>;
    fn load_qoi_header(&self) -> Result<QoiHeader, QoiError>;
}

impl<S> QoiDecode for S
where
    S: AsRef<[u8]>,
{
    fn qoi_decode(
        &self,
        channels: Option<Channels>,
        mut dest: impl AsMut<[u8]>,
    ) -> Result<(), QoiError> {
        let dest = dest.as_mut();
        let header = QoiHeader::new_from_slice(self.as_ref())?;
        let channels = channels.unwrap_or(header.channels);

        if dest.as_ref().len() < header.raw_image_size(channels) {
            return Err(QoiError::OutputTooSmall);
        }

        let mut cache = [Pixel::default(); 64];
        let mut run = 0u16;
        let padding_pos = self.as_ref().len() - Qoi::PADDING as usize;
        let mut pixel = Pixel::new(0, 0, 0, 255);
        let mut pos = 0;
        let src = &self.as_ref()[Qoi::HEADER_SIZE..];

        for chunk in dest.chunks_exact_mut(channels.len() as usize) {
            if run > 0 {
                run -= 1;
            } else if pos < padding_pos as usize {
                let b1 = src[pos];
                pos += 1;

                if b1 & Qoi::MASK_2 == Qoi::INDEX {
                    pixel = cache[(b1 ^ Qoi::INDEX) as usize];
                } else if b1 & Qoi::MASK_3 == Qoi::RUN_8 {
                    run = (b1 & 0x1f) as u16;
                } else if b1 & Qoi::MASK_3 == Qoi::RUN_16 {
                    let b2 = src[pos];
                    pos += 1;
                    run = ((((b1 & 0x1f) as u16) << 8) | b2 as u16) + 32;
                } else if (b1 & Qoi::MASK_2) == Qoi::DIFF_8 {
                    pixel.modify_r(((b1 >> 4) & 0x03) as i8 - 2);
                    pixel.modify_g(((b1 >> 2) & 0x03) as i8 - 2);
                    pixel.modify_b((b1 & 0x03) as i8 - 2);
                } else if (b1 & Qoi::MASK_3) == Qoi::DIFF_16 {
                    let b2 = src[pos];
                    pos += 1;
                    pixel.modify_r((b1 & 0x1f) as i8 - 16);
                    pixel.modify_g((b2 >> 4) as i8 - 8);
                    pixel.modify_b((b2 & 0x0f) as i8 - 8);
                } else if (b1 & Qoi::MASK_4) == Qoi::DIFF_24 {
                    let b2 = src[pos];
                    pos += 1;
                    let b3 = src[pos];
                    pos += 1;

                    pixel.modify_r((((b1 & 0x0f) << 1) | (b2 >> 7)) as i8 - 16);
                    pixel.modify_g(((b2 & 0x7c) >> 2) as i8 - 16);
                    pixel.modify_b((((b2 & 0x03) << 3) | ((b3 & 0xe0) >> 5)) as i8 - 16);
                    pixel.modify_a((b3 & 0x1f) as i8 - 16);
                } else if (b1 & Qoi::MASK_4) == Qoi::COLOR {
                    if b1 & 8 > 0 {
                        pixel.r = src[pos];
                        pos += 1;
                    }

                    if b1 & 4 > 0 {
                        pixel.g = src[pos];
                        pos += 1;
                    }

                    if b1 & 2 > 0 {
                        pixel.b = src[pos];
                        pos += 1;
                    }

                    if b1 & 1 > 0 {
                        pixel.a = src[pos];
                        pos += 1;
                    }
                }

                cache[pixel.cache_index()] = pixel;
            }

            chunk[0] = pixel.r;
            chunk[1] = pixel.g;
            chunk[2] = pixel.b;

            if channels.len() == 4 {
                chunk[3] = pixel.a;
            }
        }

        Ok(())
    }

    fn qoi_decode_to_vec(&self, channels: Option<Channels>) -> Result<Vec<u8>, QoiError> {
        let mut dest = Vec::new();
        let header = QoiHeader::new_from_slice(self.as_ref())?;
        let channels = channels.unwrap_or(header.channels);
        dest.resize(header.raw_image_size(channels), 0);
        self.qoi_decode(Some(channels), &mut dest)?;
        Ok(dest)
    }

    fn load_qoi_header(&self) -> Result<QoiHeader, QoiError> {
        QoiHeader::new_from_slice(self.as_ref())
    }
}

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
