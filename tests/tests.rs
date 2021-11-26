use qoi::{Channels, QoiDecode, QoiEncode};

fn compare_bytes(l: &[u8], r: &[u8]) {
    assert_eq!(l.len(), r.len());
    for i in 0..l.len() {
        if l[i] != r[i] {
            panic!("Byte {} doesn't match: {} != {}", i, l[i], r[i]);
        }
    }
}

#[test]
fn decode_three_channels() {
    let encoded = include_bytes!("../tests/three.qoi");
    let expected = include_bytes!("../tests/three.raw");

    let header = encoded.load_qoi_header().unwrap();
    assert_eq!(header.width(), 572);
    assert_eq!(header.height(), 354);
    assert_eq!(header.raw_image_size(Channels::Three), expected.len());

    let mut decoded = Vec::new();
    decoded.resize(header.raw_image_size(Channels::Three), 0);

    encoded.qoi_decode(Channels::Three, &mut decoded).unwrap();
    compare_bytes(expected, decoded.as_slice());
}

#[test]
fn decode_four_channels() {
    let encoded = include_bytes!("../tests/four.qoi");
    let expected = include_bytes!("../tests/four.raw");

    let header = encoded.load_qoi_header().unwrap();
    assert_eq!(header.width(), 572);
    assert_eq!(header.height(), 354);
    assert_eq!(header.raw_image_size(Channels::Four), expected.len());

    let mut decoded = Vec::new();
    decoded.resize(expected.len(), 0);

    encoded.qoi_decode(Channels::Four, &mut decoded).unwrap();
    compare_bytes(expected, decoded.as_slice());
}

#[test]
fn encode_three_channels() {
    let expected = include_bytes!("../tests/three.qoi");
    let raw = include_bytes!("../tests/three.raw");

    let mut encoded = Vec::new();
    encoded.resize(expected.len(), 0);

    raw.qoi_encode(572, 354, Channels::Three, &mut encoded)
        .unwrap();

    compare_bytes(expected, &encoded);
}

#[test]
fn encode_four_channels() {
    let expected = include_bytes!("../tests/four.qoi");
    let raw = include_bytes!("../tests/four.raw");

    let mut encoded = Vec::new();
    encoded.resize(expected.len(), 0);

    raw.qoi_encode(572, 354, Channels::Four, &mut encoded)
        .unwrap();

    compare_bytes(expected, &encoded);
}
