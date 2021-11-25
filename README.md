**This is not working yet. It's still in progress.**

A Rust implemention of the “Quite OK Image” format for fast, lossless image
compression.

See [Phoboslab's original C implementation](https://github.com/phoboslab/qoi) for more details.

## License

Apache-2.0 OR MIT.

## To do

- Make integer casts more strict to prevent overflows
- Test cases
- Benchmarks
- Fuzzing
- Replace unsafe without reducing performance
- Maybe make it generic over `Write+Seek` and the number of channels.
- Examples
