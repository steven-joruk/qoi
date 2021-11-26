use criterion::{criterion_group, criterion_main, Criterion};
use qoi::{QoiDecode, QoiEncode};

pub fn criterion_benchmark(c: &mut Criterion) {
    let three_raw = include_bytes!("../tests/three.raw");
    let three_encoded = include_bytes!("../tests/three.qoi");
    let four_raw = include_bytes!("../tests/four.raw");
    let four_encoded = include_bytes!("../tests/four.qoi");

    let mut buffer = Vec::new();
    buffer.resize(three_raw.len(), 0);

    c.bench_function("decode 3 channels", |b| {
        b.iter(|| {
            three_encoded
                .qoi_decode(qoi::Channels::Three, buffer.as_mut_slice())
                .unwrap()
        })
    });

    buffer.resize(four_raw.len(), 0);

    c.bench_function("decode 4 channels", |b| {
        b.iter(|| {
            four_encoded
                .qoi_decode(qoi::Channels::Four, buffer.as_mut_slice())
                .unwrap()
        })
    });

    c.bench_function("encode 3 channels", |b| {
        b.iter(|| {
            three_raw
                .qoi_encode(572, 354, qoi::Channels::Three, &mut buffer)
                .unwrap();
        })
    });

    c.bench_function("encode 4 channels", |b| {
        b.iter(|| {
            four_raw
                .qoi_encode(572, 354, qoi::Channels::Four, &mut buffer)
                .unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
