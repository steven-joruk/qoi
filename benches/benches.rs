use criterion::{criterion_group, criterion_main, Criterion};
use qoi::{QoiDecode, QoiEncode};

pub fn three_channels(c: &mut Criterion) {
    let raw = include_bytes!("../tests/three.raw");
    let encoded = include_bytes!("../tests/three.qoi");
    let header = encoded.load_qoi_header().unwrap();

    c.bench_function("decode 3 channels", |b| {
        b.iter(|| encoded.qoi_decode_to_vec(qoi::Channels::Three).unwrap())
    });

    c.bench_function("encode 3 channels", |b| {
        b.iter(|| {
            raw.qoi_encode_to_vec(header.width(), header.height(), qoi::Channels::Three)
                .unwrap();
        })
    });
}

pub fn four_channels(c: &mut Criterion) {
    let raw = include_bytes!("../tests/four.raw");
    let encoded = include_bytes!("../tests/four.qoi");
    let header = encoded.load_qoi_header().unwrap();

    c.bench_function("decode 4 channels", |b| {
        b.iter(|| encoded.qoi_decode_to_vec(qoi::Channels::Four).unwrap())
    });

    c.bench_function("encode 4 channels", |b| {
        b.iter(|| {
            raw.qoi_encode_to_vec(header.width(), header.height(), qoi::Channels::Four)
                .unwrap();
        })
    });
}

criterion_group!(benches, three_channels, four_channels);
criterion_main!(benches);
