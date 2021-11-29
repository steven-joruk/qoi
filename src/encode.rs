use crate::{Channels, FallibleWriter, Pixel, Qoi, QoiError, QoiHeader};

trait IsBetween: PartialOrd
where
    Self: Sized,
{
    #[inline(always)]
    fn is_between(&self, low: Self, high: Self) -> bool {
        *self >= low && *self <= high
    }
}

impl IsBetween for i16 {}

#[inline(always)]
fn write_run(writer: &mut FallibleWriter, run: &mut u16) -> Result<(), QoiError> {
    if *run < 33 {
        *run -= 1;
        writer.write(Qoi::RUN_8 | (*run as u8))?;
    } else {
        *run -= 33;
        writer.write(Qoi::RUN_16 | ((*run >> 8u16) as u8))?;
        writer.write(*run as u8)?;
    }

    *run = 0;

    Ok(())
}

#[inline(always)]
fn can_diff_8(dr: i16, dg: i16, db: i16, da: i16) -> bool {
    da == 0 && dr.is_between(-2, 1) && dg.is_between(-2, 1) && db.is_between(-2, 1)
}

#[inline(always)]
fn diff_8(dr: i16, dg: i16, db: i16) -> u8 {
    Qoi::DIFF_8 | ((dr + 2) << 4) as u8 | ((dg + 2) << 2) as u8 | (db + 2) as u8
}

#[inline(always)]
fn can_diff_16(dr: i16, dg: i16, db: i16, da: i16) -> bool {
    da == 0 && dr.is_between(-16, 15) && dg.is_between(-8, 7) && db.is_between(-8, 7)
}

#[inline(always)]
fn diff_16(dr: i16, dg: i16, db: i16, writer: &mut FallibleWriter) -> Result<(), QoiError> {
    writer.write(Qoi::DIFF_16 | (dr + 16) as u8)?;
    writer.write(((dg + 8) << 4) as u8 | (db + 8) as u8)
}

#[inline(always)]
fn can_diff_24(dr: i16, dg: i16, db: i16, da: i16) -> bool {
    dr.is_between(-16, 15)
        && dg.is_between(-16, 15)
        && db.is_between(-16, 15)
        && da.is_between(-16, 15)
}

#[inline(always)]
fn diff_24(
    dr: i16,
    dg: i16,
    db: i16,
    da: i16,
    writer: &mut FallibleWriter,
) -> Result<(), QoiError> {
    writer.write(Qoi::DIFF_24 | ((dr + 16) >> 1) as u8)?;
    writer.write(((dr + 16) << 7) as u8 | ((dg + 16) << 2) as u8 | ((db + 16) >> 3) as u8)?;
    writer.write(((db + 16) << 5) as u8 | (da + 16) as u8)
}

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
        let src = self.as_ref();
        let header = QoiHeader::new(width, height, channels, colour_space);
        let mut writer = FallibleWriter::new(dest.as_mut());

        let mut cache = [Pixel::default(); 64];
        let mut previous_pixel = Pixel::new(0, 0, 0, 255);
        let mut run = 0u16;
        let raw_image_size = header.raw_image_size(channels);
        if raw_image_size < (channels.len() as usize) || src.len() < raw_image_size {
            return Err(QoiError::InputSize);
        }
        let src = &src[0..raw_image_size];
        let last_chunk_index = src.len() / channels.len() as usize - 1;

        writer.write_slice(&header.to_array())?;

        for (index, chunk) in src.chunks_exact(channels.len() as usize).enumerate() {
            let a = if channels.len() == 4 { chunk[3] } else { 255 };
            let pixel = Pixel::new(chunk[0], chunk[1], chunk[2], a);

            if pixel == previous_pixel {
                run += 1;

                if run == 0x2020 || index == last_chunk_index {
                    write_run(&mut writer, &mut run)?;
                }
            } else {
                if run > 0 {
                    write_run(&mut writer, &mut run)?;
                }

                let cache_index = pixel.cache_index();

                if pixel == *cache.get(cache_index).ok_or(QoiError::CacheIndex)? {
                    writer.write(Qoi::INDEX | (cache_index as u8))?;
                } else {
                    *(cache.get_mut(cache_index).ok_or(QoiError::CacheIndex)?) = pixel;

                    let dr = pixel.r as i16 - previous_pixel.r as i16;
                    let dg = pixel.g as i16 - previous_pixel.g as i16;
                    let db = pixel.b as i16 - previous_pixel.b as i16;
                    let da = pixel.a as i16 - previous_pixel.a as i16;

                    if can_diff_8(dr, dg, db, da) {
                        writer.write(diff_8(dr, dg, db))?;
                    } else if can_diff_16(dr, dg, db, da) {
                        diff_16(dr, dg, db, &mut writer)?;
                    } else if can_diff_24(dr, dg, db, da) {
                        diff_24(dr, dg, db, da, &mut writer)?;
                    } else {
                        let mut command = Qoi::COLOR;

                        // The command is written last to avoid extra branches.
                        let command_pos = writer.pos;
                        writer.pos += 1;

                        if dr != 0 {
                            command |= 8;
                            writer.write(pixel.r)?;
                        }

                        if dg != 0 {
                            command |= 4;
                            writer.write(pixel.g)?;
                        }

                        if db != 0 {
                            command |= 2;
                            writer.write(pixel.b)?;
                        }

                        if da != 0 {
                            command |= 1;
                            writer.write(pixel.a)?;
                        }

                        writer.write_at(command_pos, command)?;
                    }
                }

                previous_pixel = pixel;
            }
        }

        writer.write_slice(&[0; Qoi::PADDING_SIZE as usize])?;

        Ok(writer.pos)
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
            .saturating_add(Qoi::PADDING_SIZE as usize);

        if size > Qoi::MAX_SIZE {
            return Err(QoiError::TooBig);
        }

        let mut dest = Vec::new();
        dest.resize(size, 0);

        let actual_size =
            self.qoi_encode(width, height, channels, colour_space, dest.as_mut_slice())?;
        dest.resize(actual_size, 0);

        Ok(dest)
    }
}
