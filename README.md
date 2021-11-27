[![docs.rs](https://img.shields.io/crates/v/qoi)](https://crates.io/crates/qoi)
[![docs.rs](https://img.shields.io/docsrs/qoi)](https://docs.rs/qoi)

A Rust implemention of the “Quite OK Image” format for fast, lossless image
compression.

See [Phoboslab's original C implementation](https://github.com/phoboslab/qoi) for more details.

## License

Apache-2.0 OR MIT.

## To do

- Add tests for decoding using 3 and 4 channels where the source has the opposite
  number of channels.
- Include benchmarks for the C implementation
- Make integer casts more strict to prevent overflows
- More tests
- Fuzzing

## Generating test images

You'll need to build the C qoiconv utility with this hack applied:

```diff
diff --git a/qoiconv.c b/qoiconv.c
index 6e7ad36..c91d2e0 100644
--- a/qoiconv.c
+++ b/qoiconv.c
@@ -84,7 +84,12 @@ int main(int argc, char **argv) {
 			.channels = channels,
 			.colorspace = QOI_SRGB
 		});
-	}
+	} else if (STR_ENDS_WITH(argv[2], ".raw")) {
+              encoded = 1;
+              FILE* fp = fopen(argv[2], "wb");
+              fwrite(pixels, w * h * channels, 1, fp);
+              fclose(fp);
+        }

 	if (!encoded) {
 		printf("Couldn't write/encode %s\n", argv[2]);
```

Copy it to `cqoiconv`

Download https://phoboslab.org/files/qoibench/images.tar

Run `./create_tests.sh <path to images.tar>`
