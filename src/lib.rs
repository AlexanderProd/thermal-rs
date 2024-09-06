//! Library to process thermal images from FLIR cameras.
//!
//! This crate provides two functionalities:
//!
//! 1. Compute [temperature] from raw sensor values and
//! ambient parameters (typically stored as metadata in the
//! image). The code is a port of the [Thermimage R
//! library] and its [python port][read_thermal.py].
//!
//! 2. [Parse parameters](image::ThermalImage) and raw
//! sensor values from image metadata. Supports [parsing
//! R-JPEGs][parsing-rjpeg] with FFF encoding of Flir
//! parameters, and [parsing ExifTool][parsing-exiftool]
//! generated JSON (output from `exiftool -b -j`).
//!
//! # Usage
//!
//! Obtaining pixel-wise temperature values involves (1)
//! extracting the raw sensor values, and the conversion
//! parameters from image metadata; and (2) converting the
//! raw values to temperature values.
//!
//! ## Extracting values and parameters
//!
//! The crate can directly parse radiometric R-JPEGs from
//! Flir cameras. This is an (incomplete) port of the
//! relevant parts of the excellent [ExifTool] by Phil
//! Harvey and currently supports R-JPEGs with FFF encoded
//! data. Refer
//! [`try_from_rjpeg_path`][ThermalImage::try_from_rjpeg_path]
//! for more info.
//!
//! ```rust
//! # fn test_compile() -> anyhow::Result<()> {
//! use thermal::ThermalImage;
//! let image = ThermalImage::try_from_rjpeg_path("image.jpg")?;
//! # Ok(())
//! # }
//! ```
//!
//! The crate can also parse the JSON output from ExifTool
//! via `exiftool -b -j` via [`ThermalExiftoolJson`] and
//! [`serde_json`]. This can then be converted to a
//! `ThermalImage`.
//!
//! ```rust
//! # fn test_compile() -> anyhow::Result<()> {
//! use std::{convert::TryInto, fs::File, io::BufReader};
//! use thermal::{ThermalExiftoolJson, ThermalImage};
//!
//! let json: ThermalExiftoolJson = serde_json::from_reader(
//!     BufReader::new(File::open("metadata.json")?)
//! )?;
//! let image: ThermalImage = json.try_into()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Converting sensor values to temperatures
//!
//! The raw sensor values in [`ThermalImage::image`] can be
//! converted to temperature in celicius using
//! [`raw_to_temp`][crate::temperature::ThermalSettings::raw_to_temp]
//! method on [`ThermalImage::settings`]. This is a port of
//! the [Thermimage R library] and its [python
//! port][read_thermal.py].
//!
//! [read_thermal.py]: //github.com/Nervengift/read_thermal.py/blob/master/flir_image_extractor.py
//! [Thermimage R library]: //github.com/gtatters/Thermimage/blob/master/R/raw2temp.R
//! [ExifTool]: //exiftool.org
//! [parsing-rjpeg]: crate::image::ThermalImage::try_from_rjpeg
//! [parsing-exiftool]: crate::image::ThermalExiftoolJson

#[macro_use]
mod parse;
pub(crate) mod flir;

pub mod image;
pub mod temperature;

pub mod stats;

pub use crate::image::ThermalExiftoolJson;
pub use crate::image::ThermalImage;

pub mod dji;

pub mod cli;
