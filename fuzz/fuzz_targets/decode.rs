#![no_main]

use libfuzzer_sys::fuzz_target;
use qoi::{Channels, QoiDecode};

fuzz_target!(|data: &[u8]| {
    if data.len() < 1 {
        return;
    }

    let channels = match data[0] {
        0 => Some(Channels::Three),
        1 => Some(Channels::Four),
        _ => None,
    };

    let _result = (&data[1..]).qoi_decode_to_vec(channels);
});
