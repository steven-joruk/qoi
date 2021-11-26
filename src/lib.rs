use std::{
    error::Error,
    fmt::Display,
    io::{Cursor, Seek, SeekFrom, Write},
};

#[derive(Debug)]
pub enum QoiError {
    InputTooSmall,
    OutputTooSmall,
    InvalidHeader,
    Io(std::io::Error),
}

impl Error for QoiError {}

impl Display for QoiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QoiError::InputTooSmall => f.write_str("The input is too small"),
            QoiError::OutputTooSmall => f.write_str("The output buffer is too small"),
            QoiError::InvalidHeader => f.write_str("The header is invalid"),
            QoiError::Io(inner) => {
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

impl Channels {
    fn len(&self) -> u8 {
        match self {
            Self::Three => 3,
            Self::Four => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C, packed)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Default for Pixel {
    fn default() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }
}

impl Pixel {
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
    const HEADER_SIZE: usize = 12;
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

#[repr(C, packed)]
pub struct QoiHeader {
    width: [u8; 2],
    height: [u8; 2],
    encoded_size_including_padding: [u8; 4],
}

impl QoiHeader {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width: width.to_be_bytes(),
            height: height.to_be_bytes(),
            encoded_size_including_padding: [0u8; 4],
        }
    }

    fn to_header(&self, encoded_size: u32) -> [u8; Qoi::HEADER_SIZE] {
        let mut dest = [0u8; Qoi::HEADER_SIZE];

        dest[0..4].copy_from_slice(b"qoif");
        dest[4..6].copy_from_slice(&self.width);
        dest[6..8].copy_from_slice(&self.height);
        dest[8..12].copy_from_slice(&encoded_size.to_be_bytes());

        dest
    }

    pub fn width(&self) -> u16 {
        u16::from_be_bytes(self.width)
    }

    pub fn height(&self) -> u16 {
        u16::from_be_bytes(self.height)
    }

    /// The size of the image in its raw, uncompressed format.
    pub fn raw_image_size(&self, channels: Channels) -> usize {
        self.width() as usize * self.height() as usize * channels.len() as usize
    }

    fn encoded_size_including_padding(&self) -> usize {
        u32::from_be_bytes(self.encoded_size_including_padding) as usize
    }

    fn encoded_size(&self) -> usize {
        self.encoded_size_including_padding() as usize - Qoi::PADDING as usize
    }

    fn new_from_slice(input: &[u8]) -> Result<Self, QoiError> {
        if input.len() < Qoi::HEADER_SIZE as usize {
            return Err(QoiError::InputTooSmall);
        }

        if &input[0..4] != b"qoif" {
            return Err(QoiError::InvalidHeader);
        }

        let header = QoiHeader {
            width: input[4..6]
                .try_into()
                .map_err(|_| QoiError::InputTooSmall)?,
            height: input[6..8]
                .try_into()
                .map_err(|_| QoiError::InputTooSmall)?,
            encoded_size_including_padding: input[8..12]
                .try_into()
                .map_err(|_| QoiError::InputTooSmall)?,
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
        width: u16,
        height: u16,
        channels: Channels,
        dest: impl AsMut<[u8]>,
    ) -> Result<usize, QoiError>;
}

impl<S> QoiEncode for S
where
    S: AsRef<[u8]>,
{
    fn qoi_encode(
        &self,
        width: u16,
        height: u16,
        channels: Channels,
        mut dest: impl AsMut<[u8]>,
    ) -> Result<usize, QoiError> {
        let mut cursor = Cursor::new(dest.as_mut());

        // This will be written later once the encoded size is known.
        cursor.seek(SeekFrom::Start(Qoi::HEADER_SIZE as u64))?;

        let src = self.as_ref();
        let mut cache = [Pixel::default(); 64];
        let mut previous_pixel = Pixel::default();
        let mut run = 0u16;

        if src.len() < width as usize * height as usize * channels.len() as usize {
            return Err(QoiError::OutputTooSmall);
        }

        for pos in (0..src.len()).step_by(channels.len() as usize) {
            let a = if channels.len() == 4 {
                src[pos + 3]
            } else {
                255
            };

            let pixel = Pixel::new(src[pos], src[pos + 1], src[pos + 2], a);

            if pixel == previous_pixel {
                run += 1;
            }

            if run > 0
                && (run == 0x2020
                    || pixel != previous_pixel
                    || pos == src.len() - channels.len() as usize)
            {
                if run < 33 {
                    run -= 1;
                    cursor.write_all(&[Qoi::RUN_8 | (run as u8)]).unwrap();
                } else {
                    run -= 33;
                    cursor
                        .write_all(&[Qoi::RUN_16 | ((run >> 8u16) as u8), run as u8])
                        .unwrap();
                }

                run = 0;
            }

            if pixel != previous_pixel {
                let cache_index = pixel.cache_index();

                if pixel == cache[cache_index] {
                    cursor
                        .write_all(&[Qoi::INDEX | (cache_index as u8)])
                        .unwrap();
                } else {
                    cache[cache_index] = pixel;

                    let dr = pixel.r as i16 - previous_pixel.r as i16;
                    let dg = pixel.g as i16 - previous_pixel.g as i16;
                    let db = pixel.b as i16 - previous_pixel.b as i16;
                    let da = pixel.a as i16 - previous_pixel.a as i16;

                    if da == 0
                        && dr.is_between(-1, 2)
                        && dg.is_between(-1, 2)
                        && db.is_between(-1, 2)
                    {
                        cursor
                            .write_all(&[(Qoi::DIFF_8
                                | ((dr + 1) << 4) as u8
                                | ((dg + 1) << 2) as u8
                                | (db + 1) as u8)])
                            .unwrap();
                    } else if da == 0
                        && dr.is_between(-15, 16)
                        && dg.is_between(-7, 8)
                        && db.is_between(-7, 8)
                    {
                        cursor
                            .write_all(&[
                                Qoi::DIFF_16 | (dr + 15) as u8,
                                ((dg + 7) << 4) as u8 | (db + 7) as u8,
                            ])
                            .unwrap();
                    } else if dr.is_between(-15, 16)
                        && dg.is_between(-15, 16)
                        && db.is_between(-15, 16)
                        && da.is_between(-15, 16)
                    {
                        cursor
                            .write_all(&[
                                Qoi::DIFF_24 | ((dr + 15) >> 1) as u8,
                                ((dr + 15) << 7) as u8
                                    | ((dg + 15) << 2) as u8
                                    | ((db + 15) >> 3) as u8,
                                ((db + 15) << 5) as u8 | (da + 15) as u8,
                            ])
                            .unwrap();
                    } else {
                        let command = Qoi::COLOR
                            | if dr != 0 { 8 } else { 0 }
                            | if dg != 0 { 4 } else { 0 }
                            | if db != 0 { 2 } else { 0 }
                            | if da != 0 { 1 } else { 0 };

                        cursor.write_all(&[command]).unwrap();

                        if dr != 0 {
                            cursor.write_all(&[pixel.r]).unwrap();
                        }

                        if dg != 0 {
                            cursor.write_all(&[pixel.g]).unwrap();
                        }

                        if db != 0 {
                            cursor.write_all(&[pixel.b]).unwrap();
                        }

                        if da != 0 {
                            cursor.write_all(&[pixel.a]).unwrap();
                        }
                    }
                }
            }

            previous_pixel = pixel;
        }

        cursor.write_all(&[0u8; Qoi::PADDING as usize])?;

        let config = QoiHeader::new(width, height);

        let encoded_size = cursor.position() as u32 - Qoi::HEADER_SIZE as u32;
        cursor.seek(SeekFrom::Start(0))?;
        cursor.write_all(&config.to_header(encoded_size)).unwrap();

        Ok(encoded_size as usize)
    }
}

pub trait QoiDecode {
    fn qoi_decode(&self, channels: Channels, dest: impl AsMut<[u8]>) -> Result<(), QoiError>;
    fn qoi_decode_to_vec(&self, channels: Channels) -> Result<Vec<u8>, QoiError>;
    fn load_qoi_header(&self) -> Result<QoiHeader, QoiError>;
}

impl<S> QoiDecode for S
where
    S: AsRef<[u8]>,
{
    fn qoi_decode(&self, channels: Channels, mut dest: impl AsMut<[u8]>) -> Result<(), QoiError> {
        let dest = dest.as_mut();

        let header = QoiHeader::new_from_slice(self.as_ref())?;
        if self.as_ref().len()
            < header.encoded_size_including_padding() as usize + Qoi::HEADER_SIZE as usize
        {
            return Err(QoiError::InputTooSmall);
        }

        if dest.as_ref().len() < header.raw_image_size(channels) {
            return Err(QoiError::OutputTooSmall);
        }

        let mut cache = [Pixel::default(); 64];
        let mut run = 0u16;
        let padding_pos = header.encoded_size() + Qoi::HEADER_SIZE;
        let mut pixel = Pixel::default();
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
                    pixel.modify_r(((b1 >> 4) & 0x03) as i8 - 1);
                    pixel.modify_g(((b1 >> 2) & 0x03) as i8 - 1);
                    pixel.modify_b((b1 & 0x03) as i8 - 1);
                } else if (b1 & Qoi::MASK_3) == Qoi::DIFF_16 {
                    let b2 = src[pos];
                    pos += 1;
                    pixel.modify_r((b1 & 0x1f) as i8 - 15);
                    pixel.modify_g((b2 >> 4) as i8 - 7);
                    pixel.modify_b((b2 & 0x0f) as i8 - 7);
                } else if (b1 & Qoi::MASK_4) == Qoi::DIFF_24 {
                    let b2 = src[pos];
                    pos += 1;
                    let b3 = src[pos];
                    pos += 1;

                    pixel.modify_r((((b1 & 0x0f) << 1) | (b2 >> 7)) as i8 - 15);
                    pixel.modify_g(((b2 & 0x7c) >> 2) as i8 - 15);
                    pixel.modify_b((((b2 & 0x03) << 3) | ((b3 & 0xe0) >> 5)) as i8 - 15);
                    pixel.modify_a((b3 & 0x1f) as i8 - 15);
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

    fn qoi_decode_to_vec(&self, channels: Channels) -> Result<Vec<u8>, QoiError> {
        let mut dest = Vec::new();
        let header = QoiHeader::new_from_slice(self.as_ref())?;
        dest.resize(header.raw_image_size(channels), 0);
        self.qoi_decode(channels, &mut dest)?;
        Ok(dest)
    }

    fn load_qoi_header(&self) -> Result<QoiHeader, QoiError> {
        QoiHeader::new_from_slice(self.as_ref())
    }
}
