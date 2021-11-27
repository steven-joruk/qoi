use criterion::{criterion_group, criterion_main, Criterion};
use qoi::{QoiDecode, QoiEncode};

pub fn four_channels(c: &mut Criterion) {
    let raw = include_bytes!("../images/misc/dice.raw");
    let encoded = include_bytes!("../images/misc/dice.qoi");
    let header = encoded.load_qoi_header().unwrap();

    c.bench_function("decode 4 channels", |b| {
        b.iter(|| encoded.qoi_decode_to_vec(None).unwrap())
    });

    c.bench_function("encode 4 channels", |b| {
        b.iter(|| {
            raw.qoi_encode_to_vec(header.width(), header.height(), qoi::Channels::Four, 0)
                .unwrap();
        })
    });
}

criterion_group!(benches, four_channels);
criterion_main!(benches);
