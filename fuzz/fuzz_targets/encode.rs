#![no_main]

use libfuzzer_sys::fuzz_target;
use qoi::{Channels, QoiEncode};
use std::convert::TryInto;

fuzz_target!(|data: &[u8]| {
    if data.len() < 10 {
        return;
    }

    let width = u32::from_le_bytes((&data[0..4]).try_into().unwrap());
    let height = u32::from_le_bytes((&data[4..8]).try_into().unwrap());

    let channels = if data[8] < 5 {
        Channels::Three
    } else {
        Channels::Four
    };

    let colour_space = data[9];

    let _result = (&data[10..]).qoi_encode_to_vec(width, height, channels, colour_space);
});
