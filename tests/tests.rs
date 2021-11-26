use qoi::{Channels, QoiDecode, QoiEncode, QoiError};

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
    let decoded = encoded.qoi_decode_to_vec(Channels::Three).unwrap();
    compare_bytes(expected, decoded.as_slice());
}

#[test]
fn decode_four_channels() {
    let encoded = include_bytes!("../tests/four.qoi");
    let expected = include_bytes!("../tests/four.raw");
    let decoded = encoded.qoi_decode_to_vec(Channels::Four).unwrap();
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

#[test]
fn header_magic() {
    assert!(matches!(
        b"boif1234112341234123423412341234"
            .qoi_decode_to_vec(Channels::Three)
            .unwrap_err(),
        QoiError::InvalidHeader
    ));
}

#[test]
fn buffer_size_errors() {
    let mut buffer = Vec::new();
    buffer.resize(1024, 0);

    let error = b"qoif1234123412341234"
        .qoi_decode(Channels::Three, &mut buffer)
        .unwrap_err();
    assert!(matches!(error, QoiError::InputTooSmall));

    let error = b"qoif"
        .qoi_decode(Channels::Three, &mut buffer)
        .unwrap_err();
    assert!(matches!(error, QoiError::InputTooSmall));
}
