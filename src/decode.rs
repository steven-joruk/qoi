use crate::{Channels, Pixel, Qoi, QoiError, QoiHeader};

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

        if self.as_ref().len() < Qoi::HEADER_SIZE + Qoi::PADDING as usize {
            return Err(QoiError::InputSize);
        }

        let mut cache = [Pixel::default(); 64];
        let mut run = 0u16;
        let padding_pos = self.as_ref().len() - Qoi::PADDING as usize;
        let mut pixel = Pixel::new(0, 0, 0, 255);
        let mut pos = 0;
        let src = &self.as_ref()[Qoi::HEADER_SIZE..];

        #[inline]
        fn get(buf: &[u8], pos: usize) -> Result<u8, QoiError> {
            Ok(*buf.get(pos).ok_or(QoiError::InputSize)?)
        }

        for chunk in dest.chunks_exact_mut(channels.len() as usize) {
            if pos >= src.len() {
                return Err(QoiError::InputSize);
            }

            if run > 0 {
                run -= 1;
            } else if pos < padding_pos as usize {
                let b1 = get(src, pos)?;
                pos += 1;

                if b1 & Qoi::MASK_2 == Qoi::INDEX {
                    pixel = cache[(b1 ^ Qoi::INDEX) as usize];
                } else if b1 & Qoi::MASK_3 == Qoi::RUN_8 {
                    run = (b1 & 0x1f) as u16;
                } else if b1 & Qoi::MASK_3 == Qoi::RUN_16 {
                    let b2 = get(src, pos)?;
                    pos += 1;
                    run = ((((b1 & 0x1f) as u16) << 8) | b2 as u16) + 32;
                } else if (b1 & Qoi::MASK_2) == Qoi::DIFF_8 {
                    pixel.modify_r(((b1 >> 4) & 0x03) as i8 - 2);
                    pixel.modify_g(((b1 >> 2) & 0x03) as i8 - 2);
                    pixel.modify_b((b1 & 0x03) as i8 - 2);
                } else if (b1 & Qoi::MASK_3) == Qoi::DIFF_16 {
                    let b2 = get(src, pos)?;
                    pos += 1;
                    pixel.modify_r((b1 & 0x1f) as i8 - 16);
                    pixel.modify_g((b2 >> 4) as i8 - 8);
                    pixel.modify_b((b2 & 0x0f) as i8 - 8);
                } else if (b1 & Qoi::MASK_4) == Qoi::DIFF_24 {
                    let b2 = get(src, pos)?;
                    pos += 1;
                    let b3 = get(src, pos)?;
                    pos += 1;

                    pixel.modify_r((((b1 & 0x0f) << 1) | (b2 >> 7)) as i8 - 16);
                    pixel.modify_g(((b2 & 0x7c) >> 2) as i8 - 16);
                    pixel.modify_b((((b2 & 0x03) << 3) | ((b3 & 0xe0) >> 5)) as i8 - 16);
                    pixel.modify_a((b3 & 0x1f) as i8 - 16);
                } else if (b1 & Qoi::MASK_4) == Qoi::COLOR {
                    if b1 & 8 > 0 {
                        pixel.r = get(src, pos)?;
                        pos += 1;
                    }

                    if b1 & 4 > 0 {
                        pixel.g = get(src, pos)?;
                        pos += 1;
                    }

                    if b1 & 2 > 0 {
                        pixel.b = get(src, pos)?;
                        pos += 1;
                    }

                    if b1 & 1 > 0 {
                        pixel.a = get(src, pos)?;
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

        if header.raw_image_size(channels) > Qoi::MAX_SIZE {
            return Err(QoiError::TooBig);
        }

        dest.resize(header.raw_image_size(channels), 0);
        self.qoi_decode(Some(channels), &mut dest)?;
        Ok(dest)
    }

    fn load_qoi_header(&self) -> Result<QoiHeader, QoiError> {
        QoiHeader::new_from_slice(self.as_ref())
    }
}
