use qoi::{Channels, QoiDecode, QoiEncode, QoiError};

const THREE_QOI: &[u8] = include_bytes!("../images/three.qoi");
const THREE_RAW: &[u8] = include_bytes!("../images/three.raw");
const FOUR_QOI: &[u8] = include_bytes!("../images/four.qoi");
const FOUR_RAW: &[u8] = include_bytes!("../images/four.raw");

fn compare_bytes(l: &[u8], r: &[u8]) {
    for i in 0..l.len() {
        if l[i] != r[i] {
            panic!("Byte {} doesn't match: {} != {}", i, l[i], r[i]);
        }
    }
    assert_eq!(l.len(), r.len());
}

#[test]
fn decode_three_channels() {
    let decoded = THREE_QOI.qoi_decode_to_vec(Channels::Three).unwrap();
    compare_bytes(THREE_RAW, decoded.as_slice());
}

#[test]
fn decode_four_channels() {
    let decoded = FOUR_QOI.qoi_decode_to_vec(Channels::Four).unwrap();
    compare_bytes(FOUR_RAW, decoded.as_slice());
}

#[test]
fn encode_three_channels() {
    let header = THREE_QOI.load_qoi_header().unwrap();
    let encoded = THREE_RAW
        .qoi_encode_to_vec(header.width(), header.height(), Channels::Three)
        .unwrap();
    compare_bytes(THREE_QOI, &encoded);
}

#[test]
fn encode_four_channels() {
    let header = FOUR_QOI.load_qoi_header().unwrap();
    let encoded = FOUR_RAW
        .qoi_encode_to_vec(header.width(), header.height(), Channels::Four)
        .unwrap();
    compare_bytes(FOUR_QOI, &encoded);
}

#[test]
fn header_magic() {
    assert!(matches!(
        b"boif1234112341234123423412341234"
            .qoi_decode_to_vec(Channels::Three)
            .unwrap_err(),
        QoiError::IncorrectHeaderMagic
    ));
}

#[test]
fn buffer_size_errors() {
    let mut buffer = Vec::new();
    buffer.resize(1024, 0);

    let error = b"qoif1234123412341234"
        .qoi_decode(Channels::Three, &mut buffer)
        .unwrap_err();
    assert!(matches!(error, QoiError::InputSize));

    let error = b"qoif"
        .qoi_decode(Channels::Three, &mut buffer)
        .unwrap_err();
    assert!(matches!(error, QoiError::InputSmallerThanHeader));
}
