# DZI
A library and CLI utility for creating deep zoom images.

Deep-zoom images, or DZIs are a standard first published by Microsoft 
for storing and viewing very large images while being able to zoom and pan
without loading the entire image into the viewer.

See https://openseadragon.github.io/ for a reference implementation.

This crate is based on the excellent python implementation at
https://github.com/openzoom/deepzoom.py

## Installation
```bash
cargo install --force dzi
```

## Usage
```bash
dzi path/to/some/image
```
Example: `dzi ./test.jpg` will create a directory `./test_files/` 
with image tiles and a descriptor `./test_files/test.dzi`.

## Attributions
Test image taen from https://unsplash.com/photos/cbEvoHbJnIE
Much of the logic is adapted from https://github.com/openzoom/deepzoom.py 

## License
2-Clause BSD license. See LICENSE.txt
