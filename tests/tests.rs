use qoi::{Channels, QoiDecode, QoiEncode, QoiError, QoiHeader};
use std::{ffi::OsStr, fs::ReadDir, path::PathBuf};

fn compare_bytes(l: &[u8], r: &[u8]) {
    for i in 0..l.len() {
        if l[i] != r[i] {
            panic!("Byte {} doesn't match: {} != {}", i, l[i], r[i]);
        }
    }
    assert_eq!(l.len(), r.len());
}

struct TestCase {
    path: PathBuf,
    encoded: Vec<u8>,
    header: QoiHeader,
}
struct TestCaseIterator {
    read_dir: ReadDir,
}

impl TestCaseIterator {
    fn new() -> Self {
        Self {
            read_dir: std::fs::read_dir("images").unwrap(),
        }
    }
}

impl Iterator for TestCaseIterator {
    type Item = TestCase;

    fn next(&mut self) -> Option<Self::Item> {
        for entry in self.read_dir.next() {
            let entry = entry.unwrap();
            if entry.path().extension() == Some(OsStr::new(".qoi")) {
                let encoded = std::fs::read(entry.path()).unwrap();
                let header = encoded.load_qoi_header().unwrap();
                return Some(TestCase {
                    path: entry.path(),
                    encoded,
                    header,
                });
            }
        }

        None
    }
}

#[test]
fn round_trip_three_channels() {
    for case in TestCaseIterator::new() {
        println!("Testing {}", case.path.display());

        let decoded = case.encoded.qoi_decode_to_vec(Channels::Three).unwrap();

        let encoded = decoded
            .qoi_encode_to_vec(case.header.width(), case.header.height(), Channels::Three)
            .unwrap();

        compare_bytes(&encoded, &case.encoded);
    }
}

#[test]
fn round_trip_four_channels() {
    for case in TestCaseIterator::new() {
        println!("Testing {}", case.path.display());

        let decoded = case.encoded.qoi_decode_to_vec(Channels::Four).unwrap();

        let encoded = decoded
            .qoi_encode_to_vec(case.header.width(), case.header.height(), Channels::Four)
            .unwrap();

        compare_bytes(&encoded, &case.encoded);
    }
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
