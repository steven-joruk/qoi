#!/bin/bash

set -e

if [[ ! -f cqoiconv ]]; then
    echo "qoiconv from the C implementation needs to be copied here as cqoiconv"
    exit 1
fi

readonly archive="$1"
if [[ ! -f $archive ]]; then
    echo "Can't find the images archive $archive"
    exit 1
fi

rm -rf images

tar xf "$archive"

for f in images/**/*.png; do
    echo "Converting $f"
    ./cqoiconv "$f" "${f%png}qoi" || echo "failed to convert $f to qoi"
    ./cqoiconv "$f" "${f%png}raw" || echo "failed to convert $f to raw"
done
