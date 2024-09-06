//! Parse and extract raw thermal image and temperature
//! params.
use std::{convert::TryFrom, fs::read, io::Cursor, path::Path};

use anyhow::{anyhow, bail, Result};
use image::{ColorType, ImageDecoder};
use img_parts::jpeg::Jpeg;
use ndarray::Array2;
use serde_derive::*;

use crate::{flir::FlirSegment, temperature::ThermalSettings};

/// Container for the raw sensor values, and the parameters
/// of a single Flir image.
pub struct ThermalImage {
    pub settings: ThermalSettings,
    pub image: Array2<f64>,
}
impl ThermalImage {
    /// Parse a `ThermalImage` from
    /// [`Jpeg`][`img_parts::jpeg::Jpeg`].
    pub fn try_from_rjpeg(image: &Jpeg) -> Result<Self> {
        let flir_segment = FlirSegment::try_from_jpeg(image)?;
        let image = flir_segment
            .try_parse_raw_data()?
            .ok_or_else(|| anyhow!("no raw data found"))?;
        let settings: ThermalSettings = flir_segment
            .try_parse_camera_params()?
            .ok_or_else(|| anyhow!("no camera params found"))?
            .into();
        Ok(ThermalImage { image, settings })
    }

    /// Parse a `ThermalImage` from path to a R-Jpeg image file.
    pub fn try_from_rjpeg_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let image = Jpeg::from_bytes(read(path)?.into())?;
        Self::try_from_rjpeg(&image)
    }

    /// Try to convert a parsed `ThermalExiftoolJson`
    /// structure into a `ThermalImage`.
    pub fn try_from_thermal_exiftool_json(json: ThermalExiftoolJson) -> Result<Self> {
        Ok(Self {
            settings: json.settings,
            image: json.raw.thermal_image()?,
        })
    }
}

/// Parse output of `exiftool` json output.
///
/// This is the entry point for users interested in parsing
/// the output from `exiftool -j -b` on a thermal image. It
/// expects and extracts both the thermal settings, and the
/// raw image encoded as a base64 string.
#[derive(Serialize, Deserialize, Debug)]
pub struct ThermalExiftoolJson {
    #[serde(flatten)]
    pub settings: ThermalSettings,

    #[serde(flatten)]
    pub(crate) raw: ThermalRawBytes,
}
impl TryFrom<ThermalExiftoolJson> for ThermalImage {
    type Error = anyhow::Error;

    fn try_from(value: ThermalExiftoolJson) -> Result<Self> {
        Self::try_from_thermal_exiftool_json(value)
    }
}

/// Raw image bytes serialized by `exiftool` as JSON.
#[derive(Serialize, Deserialize, Debug)]
pub struct ThermalRawBytes {
    #[serde(rename = "RawThermalImageType")]
    ty: String,

    #[serde(
        rename = "RawThermalImage",
        deserialize_with = "serde_helpers::base64_bytes"
    )]
    base64_bytes: Vec<u8>,
}
impl ThermalRawBytes {
    pub fn thermal_image(&self) -> Result<Array2<f64>> {
        if self.ty != "TIFF" {
            bail!("unsupported image type: {}", self.ty);
        }

        use image::tiff::TiffDecoder;
        let decoder = TiffDecoder::new(Cursor::new(&self.base64_bytes))?;
        let (width, height) = decoder.dimensions();
        let width = width as usize;
        let height = height as usize;
        let depth = match decoder.color_type() {
            ColorType::L8 => 8,
            ColorType::L16 => 16,
            _ => bail!("supported color type: {:?}", decoder.color_type()),
        };

        use zerocopy::{AsBytes, FromBytes};
        fn image_as_float<'a, T, R>(decoder: R) -> Result<Vec<f64>>
        where
            f64: From<T>,
            T: AsBytes + FromBytes,
            R: ImageDecoder<'a>,
        {
            let (width, height) = decoder.dimensions();
            let num_pixels = (width * height) as usize;

            let mut image: Vec<u8> = vec![0; num_pixels];
            decoder.read_image(image.as_bytes_mut())?;
            Ok(image.into_iter().map(|f| f.into()).collect())
        }

        let output = if depth == 8 {
            image_as_float::<u8, _>(decoder)?
        } else if depth == 16 {
            image_as_float::<u16, _>(decoder)?
        } else {
            unreachable!("unexpected depth: {}", depth);
        };

        Ok(Array2::from_shape_vec((height, width), output)?)
    }
}

mod serde_helpers {
    use lazy_static::lazy_static;
    use regex::Regex;
    use serde::*;

    pub fn base64_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^base64:").unwrap();
        }

        use serde::de::Error;
        let str_rep = <String as Deserialize>::deserialize(de)?;

        RE.find(&str_rep).ok_or(Error::custom(
            "unexpected format: must begin with `base64:`",
        ))?;

        use base64::decode;
        let slice = &str_rep[7..];
        let bytes = decode(slice).map_err(Error::custom)?;

        Ok(bytes)
    }
}
