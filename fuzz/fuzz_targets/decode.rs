#![no_main]

use libfuzzer_sys::fuzz_target;
use qoi::QoiDecode;

fuzz_target!(|data: &[u8]| {
    data.qoi_decode_to_vec(None);
});
