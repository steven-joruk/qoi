[![docs.rs](https://img.shields.io/crates/v/qoi)](https://crates.io/crates/qoi)
[![docs.rs](https://img.shields.io/docsrs/qoi)](https://docs.rs/qoi)

A Rust implemention of the “Quite OK Image” format for fast, lossless image
compression.

See [Phoboslab's original C implementation](https://github.com/phoboslab/qoi) for more details.

## License

Apache-2.0 OR MIT.

## To do

- Automatically create .raw and .qoi test cases using the C imeplementation to
  test against: https://phoboslab.org/files/qoibench/
- Include benchmarks for the C implementation
- Make integer casts more strict to prevent overflows
- More tests
- Fuzzing
