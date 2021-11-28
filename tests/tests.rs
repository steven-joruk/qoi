use qoi::{QoiDecode, QoiEncode, QoiError, QoiHeader};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

fn compare_bytes(actual: &[u8], expected: &[u8]) {
    for i in 0..actual.len() {
        if actual[i] != expected[i] {
            panic!("Byte {} doesn't match: {} != {}", i, actual[i], expected[i]);
        }
    }
}

struct TestCase {
    path: PathBuf,
    encoded: Vec<u8>,
    raw: Vec<u8>,
    header: QoiHeader,
}

impl From<&Path> for TestCase {
    fn from(path: &Path) -> Self {
        let encoded = std::fs::read(path).unwrap();
        let header = encoded.load_qoi_header().unwrap();
        Self {
            path: path.into(),
            encoded,
            raw: std::fs::read(path.with_extension("raw")).unwrap(),
            header,
        }
    }
}

fn for_all_qoi_files(f: impl Fn(&TestCase)) {
    let root = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "images",);

    for entry in WalkDir::new(root).max_depth(5).into_iter() {
        let entry = entry.unwrap();

        if entry.path().extension() == Some(OsStr::new("qoi")) {
            let test_case = entry.path().into();
            f(&test_case)
        }
    }
}

#[test]
fn decode() {
    for_all_qoi_files(|case| {
        println!("Testing {}", case.path.display());
        let decoded = case.encoded.qoi_decode_to_vec(None).unwrap();
        compare_bytes(&decoded, &case.raw);
    });
}

#[test]
fn encode() {
    for_all_qoi_files(|case| {
        println!("Testing {}", case.path.display());

        let encoded = case
            .raw
            .qoi_encode_to_vec(
                case.header.width(),
                case.header.height(),
                case.header.channels(),
                0,
            )
            .unwrap();

        compare_bytes(&encoded, &case.encoded);
    });
}

#[test]
fn header_magic() {
    assert!(matches!(
        b"boif1234123412".qoi_decode_to_vec(None).unwrap_err(),
        QoiError::IncorrectHeaderMagic
    ));
}

#[test]
fn buffer_size_errors() {
    let mut buffer = Vec::new();
    buffer.resize(1024, 0);

    let error = b"qoif123412341".qoi_decode(None, &mut buffer).unwrap_err();
    assert!(matches!(error, QoiError::InputSmallerThanHeader));
}
