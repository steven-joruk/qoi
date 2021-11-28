use crate::{Channels, IsBetween, Pixel, Qoi, QoiError, QoiHeader};

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
        if raw_image_size < (channels.len() as usize) || src.len() < raw_image_size {
            return Err(QoiError::InputSize);
        }
        let src = &src[0..raw_image_size];

        dest[0..Qoi::HEADER_SIZE].copy_from_slice(&header.to_array());
        let mut dest_pos = Qoi::HEADER_SIZE;
        let last_chunk_index = src.len() / channels.len() as usize - 1;

        for (index, chunk) in src.chunks_exact(channels.len() as usize).enumerate() {
            let a = if channels.len() == 4 { chunk[3] } else { 255 };
            let pixel = Pixel::new(chunk[0], chunk[1], chunk[2], a);

            if pixel == previous_pixel {
                run += 1;
            }

            if run > 0 && (pixel != previous_pixel || run == 0x2020 || index == last_chunk_index) {
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
                    fn diff_16(dr: i16, dg: i16, db: i16, dest: &mut [u8]) {
                        dest[0] = Qoi::DIFF_16 | (dr + 16) as u8;
                        dest[1] = ((dg + 8) << 4) as u8 | (db + 8) as u8;
                    }

                    #[inline]
                    fn can_diff_24(dr: i16, dg: i16, db: i16, da: i16) -> bool {
                        dr.is_between(-16, 15)
                            && dg.is_between(-16, 15)
                            && db.is_between(-16, 15)
                            && da.is_between(-16, 15)
                    }

                    #[inline]
                    fn diff_24(dr: i16, dg: i16, db: i16, da: i16, dest: &mut [u8]) {
                        dest[0] = Qoi::DIFF_24 | ((dr + 16) >> 1) as u8;

                        dest[1] = ((dr + 16) << 7) as u8
                            | ((dg + 16) << 2) as u8
                            | ((db + 16) >> 3) as u8;

                        dest[2] = ((db + 16) << 5) as u8 | (da + 16) as u8;
                    }

                    if can_diff_24(dr, dg, db, da) {
                        if can_diff_8(dr, dg, db, da) {
                            dest[dest_pos] = diff_8(dr, dg, db);
                            dest_pos += 1;
                        } else if can_diff_16(dr, dg, db, da) {
                            diff_16(dr, dg, db, &mut dest[dest_pos..]);
                            dest_pos += 2;
                        } else {
                            diff_24(dr, dg, db, da, &mut dest[dest_pos..]);
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
        let size = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(channels.len() as usize)
            .saturating_add(Qoi::HEADER_SIZE as usize)
            .saturating_add(Qoi::PADDING as usize);

        if size > Qoi::MAX_SIZE {
            return Err(QoiError::TooBig);
        }

        let mut dest = Vec::new();
        dest.resize(size, 0);
        let size = self.qoi_encode(width, height, channels, colour_space, dest.as_mut_slice())?;
        dest.resize(size, 0);
        Ok(dest)
    }
}
