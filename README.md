[![Crates.io](https://img.shields.io/crates/v/thermal.svg)](https://crates.io/crates/thermal)
[![Documentation](https://docs.rs/thermal/badge.svg)](https://docs.rs/thermal)

Library and tools to process thermal images from FLIR
cameras.

# Overview

This crate provides two functionalities:

1. Compute temperature from raw sensor values and
ambient parameters (typically stored as metadata in the
image). The code is a port of the [Thermimage R
library] and its [python port][read_thermal.py].

2. Parse parameters and raw sensor values from image
metadata. Supports parsing R-JPEGs with FFF encoding of Flir
parameters, and parsing ExifTool generated JSON (output from
`exiftool -b -j`).

Please see [crate documentation][docs] for more information

# Tools

The crate also provides two handy binaries:

1. stats:  Generates temperature stats for a set of images / JSONs
2. transform: Generates temperature valued 16-bit single
   channel TIFF files for images, with value normalized to
   encode a given range of temperatures.

## License

Licensed under either of [Apache License, Version
2.0](//www.apache.org/licenses/LICENSE-2.0) or [MIT
license](//opensource.org/licenses/MIT) at your option.

Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in this crate by you,
as defined in the Apache-2.0 license, shall be dual licensed
as above, without any additional terms or conditions.

[read_thermal.py]: //github.com/Nervengift/read_thermal.py/blob/master/flir_image_extractor.py
[Thermimage R library]: //github.com/gtatters/Thermimage/blob/master/R/raw2temp.R
[ExifTool]: //exiftool.org
[docs]: //docs.rs/thermal
