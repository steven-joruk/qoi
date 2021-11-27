[![docs.rs](https://img.shields.io/crates/v/qoi)](https://crates.io/crates/qoi)
[![docs.rs](https://img.shields.io/docsrs/qoi)](https://docs.rs/qoi)

A Rust implemention of the “Quite OK Image” format for fast, lossless image
compression.

See [Phoboslab's original C implementation](https://github.com/phoboslab/qoi) for more details.

## License

Apache-2.0 OR MIT.

## To do

- Verify it against all test images from here https://phoboslab.org/files/qoibench/
- Make integer casts more strict to prevent overflows
- More tests
- Fuzzing
- Maybe make it generic over `Write+Seek` and the number of channels.
- Examples
