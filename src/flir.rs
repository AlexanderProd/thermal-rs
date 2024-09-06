//! Parse parameters from FLIR R-JPEGs.
//!
//! This is an incomplete port of relevant parts of the
//! excellent [ExifTool] by Phil Harvey and other authors.
//! Currently only supports R-JPEGs with FFF headers, with
//! 16-bit raw image, and only reads enough parameters to be
//! able to compute [`temperature`][crate::temperature] values
//! from the raw sensor values.
//!
//! For a more complete extraction of FLIR and other
//! metadata, please use [ExifTool] directly. The JSON
//! output from `exiftool -j -b` can also be used directly
//! to compute the temperature values via [ThermalExiftoolJson]
//!
//! [ExifTool]: //exiftool.org
//! [ThermalExiftoolJson]: crate::image::ThermalExiftoolJson
use anyhow::{anyhow, bail, ensure, Result};
use byteordered::{byteorder::ReadBytesExt, ByteOrdered, Endian, Endianness};
use img_parts::jpeg::{markers, Jpeg};
use ndarray::Array2;

use crate::parse::Parseable;

/// FLIR data along with parsed header.
///
/// # FLIR Header Format
///
/// The format of the FLIR segment header is explained in
/// ExifTool source code thanks to the authors' excellent
/// work.
///
/// - 0x00 - `string[4]` file format ID = "FFF\0"
/// - 0x04 - `string[16]` file creator: seen "\0","MTX IR\0","CAMCTRL\0"
/// - 0x14 - `int32u` file format version = 100
/// - 0x18 - `int32u` offset to record directory
/// - 0x1c - `int32u` number of entries in record directory
/// - 0x20 - `int32u` next free index ID = 2
/// - 0x24 - `int16u` swap pattern = 0 (?)
/// - 0x28 - `int16u[7]` spares
/// - 0x34 - `int32u[2]` reserved
/// - 0x3c - `int32u` checksum
#[derive(Debug)]
pub struct FlirSegment {
    data: Vec<u8>,
    dir: Vec<FlirRecordDirEntry>,
}

impl FlirSegment {
    /// Try to collect all the FLIR segments from an
    /// [`Jpeg`] image and parse the FLIR header from it.
    /// Returns a `FlirSegment` if both steps are
    /// successful.
    pub fn try_from_jpeg(image: &Jpeg) -> Result<Self> {
        let data = collect_flir_segment_data_from_jpeg(image)?;
        Self::try_from_segment_data(data)
    }

    /// Try to find and parse raw sensor values as a 2-D
    /// array. Returns the raw values as a 2-D array of
    /// `f64`s if found, and `None` if not found (but the
    /// parsing was otherwise successful).
    pub fn try_parse_raw_data(&self) -> Result<Option<Array2<f64>>> {
        self.dir
            .iter()
            .find_map(|e| e.try_parse_raw_data(&self.data).transpose())
            .transpose()
    }

    /// Try to find and parse the FLIR camera parameters
    /// from the data. Returns `None` if not found.
    pub fn try_parse_camera_params(&self) -> Result<Option<FlirCameraParams>> {
        self.dir
            .iter()
            .find_map(|e| e.try_parse_camera_params(&self.data).transpose())
            .transpose()
    }

    fn try_from_segment_data(data: Vec<u8>) -> Result<Self> {
        parse_as_bindings! {
            ByteOrdered::native(&data[..]),
            signature => [u8; 4],
            _creator as "creator" => [u8; 16],
            version => u32,
        }

        ensure!(&signature == b"FFF\0", "unexpected signature");

        // A heuristic to find if header data is LE or BE:
        // check that version is in [100, 200).
        let endianness = {
            let end = Endianness::native();
            if (100..200).contains(&version) {
                end
            } else {
                end.to_opposite()
            }
        };

        parse_as_bindings! {
            ByteOrdered::runtime(&data[0x18..], endianness),
            offset => u32 as usize,
            num_records => u32 as usize,
        }

        let mut reader = ByteOrdered::runtime(&data[offset..], endianness);
        let dir: Result<_> = (0..num_records)
            .map(|_| FlirRecordDirEntry::parse(&mut reader))
            .collect();
        Ok(FlirSegment { data, dir: dir? })
    }
}

/// Collect FLIR data from Jpeg APP1 segments.
///
/// # Implementation
///
/// FLIR data is stored as a collection of APP1 segments
/// with the following format:
///
/// - 0x0: signature: "FLIR\0"
/// - 0x6: segment number: zero-based idx
/// - 0x7: last segment number (= total segments - 1)
/// - 0x8..: data
///
/// The logic is exactly as in [ExifTool.pm]. We iterate
/// through all APP1 segments; check each for the signature;
/// verify the segment idx, total are consistent; and return
/// the concatenated payload.
///
/// [ExifTool.pm]: //github.com/exiftool/exiftool/blob/master/lib/Image/ExifTool.pm
fn collect_flir_segment_data_from_jpeg(image: &Jpeg) -> Result<Vec<u8>> {
    let mut flir_segments: Vec<Vec<u8>> = vec![];
    let mut num_copied = 0;
    let mut total_len = 0;

    for segment in image.segments_by_marker(markers::APP1) {
        let contents = segment.contents();
        if contents.len() < 8 || &contents[0..5] != b"FLIR\0" {
            continue;
        }

        let current_segment = contents[6] as usize;
        let total_segments = contents[7] as usize + 1;

        match flir_segments.len() {
            0 => flir_segments.resize(total_segments, vec![]),
            l if l != total_segments => bail!(
                "inconsistent count of total FLIR segments: {} != {}",
                l,
                total_segments
            ),
            l if l <= current_segment => bail!(
                "FLIR segment idx out of bounds: {} >= {}",
                current_segment,
                l
            ),
            _ => (),
        }

        let curr_seg = &mut flir_segments[current_segment];
        ensure!(
            curr_seg.is_empty(),
            "duplicate FLIR segment: idx = {}",
            current_segment
        );

        curr_seg.extend_from_slice(&contents[8..]);
        num_copied += 1;
        total_len += curr_seg.len();
    }

    ensure!(
        num_copied == flir_segments.len(),
        "expected {} FLIR segments, found only {}",
        flir_segments.len(),
        num_copied
    );

    let mut flir_data = Vec::with_capacity(total_len);
    for seg in flir_segments {
        flir_data.extend_from_slice(&seg);
    }
    Ok(flir_data)
}

// # FLIR record entry (ref 3):
// # 0x00 - int16u record type
// # 0x02 - int16u record subtype: RawData 1=BE, 2=LE, 3=PNG; 1 for other record types
// # 0x04 - int32u record version: seen 0x64,0x66,0x67,0x68,0x6f,0x104
// # 0x08 - int32u index id = 1
// # 0x0c - int32u record offset from start of FLIR data
// # 0x10 - int32u record length
// # 0x14 - int32u parent = 0 (?)
// # 0x18 - int32u object number = 0 (?)
// # 0x1c - int32u checksum: 0 for no checksum
declare_parseable_struct! {
    /// Details of a FLIR record
    #[derive(Debug)]
    pub struct FlirRecordDirEntry {
        ty => u16,
        sub_type=> u16,
        version=> u32,

        id=> u32,
        offset=> u32,
        length=> u32,

        parent=> u32,
        obj_num=> u32,
        checksum=> u32,
    }
}
impl FlirRecordDirEntry {
    /// Get the data associated with this record
    pub fn data<'a>(&self, segment: &'a [u8]) -> Result<&'a [u8]> {
        segment
            .get(self.offset as usize..(self.offset + self.length) as usize)
            .ok_or_else(|| anyhow!("unexpected EOF while reading data"))
    }

    pub fn try_parse_raw_data(&self, segment: &[u8]) -> Result<Option<Array2<f64>>> {
        if self.ty != 0x01 {
            return Ok(None);
        }
        ensure!(self.sub_type != 3, "PNG type raw data not yet supported");

        let data = self.data(segment)?;

        ensure!(
            data.len() > 6,
            "raw data record size mismatch: expected at least 6 bytes, found {}",
            data.len(),
        );

        let endianness = {
            parse_as_bindings! {
                ByteOrdered::native(&data[0..2]),
                check_val => u16,
            }
            let end = Endianness::native();
            if check_val == 2 {
                end
            } else {
                end.to_opposite()
            }
        };

        parse_as_bindings! {
            ByteOrdered::runtime(&data[2..], endianness),
            width => u16 as usize,
            height => u16 as usize,
        }

        let expected = 2 * (16 + width * height);
        ensure!(
            data.len() == expected,
            "raw data record size mismatch: expected {} bytes, found {}",
            expected,
            data.len()
        );

        let mut reader = ByteOrdered::runtime(&data[0x20..], endianness);
        let mut raw_data = Vec::with_capacity(width * height);
        for _ in 0..height {
            for _ in 0..width {
                raw_data.push(u16::parse(&mut reader)? as f64);
            }
        }

        Ok(Some(Array2::from_shape_vec((height, width), raw_data)?))
    }
    pub fn try_parse_camera_params(&self, segment: &[u8]) -> Result<Option<FlirCameraParams>> {
        if self.ty != 0x20 {
            return Ok(None);
        }

        let data = self.data(segment)?;

        ensure!(
            data.len() >= 0x384,
            "raw data record size mismatch: expected at least {} bytes, found {}",
            0x384,
            data.len()
        );

        let endianness = {
            parse_as_bindings! {
                ByteOrdered::native(&data[0..2]),
                check_val => u16,
            }
            let end = Endianness::native();
            if check_val == 2 {
                end
            } else {
                end.to_opposite()
            }
        };

        parse_as_bindings! {
            ByteOrdered::runtime(&data[0x20..], endianness),
            temperature_params => FlirTemperatureParams,
        }
        parse_as_bindings! {
            ByteOrdered::runtime(&data[0xd4..], endianness),
            camera_info => FlirCameraInfo,
        }
        parse_as_bindings! {
            ByteOrdered::runtime(&data[0x170..], endianness),
            lens_info => FlirLensInfo,
        }
        parse_as_bindings! {
            ByteOrdered::runtime(&data[0x1ec..], endianness),
            filter_info => FlirFilterInfo,
        }
        parse_as_bindings! {
            ByteOrdered::runtime(&data[0x308..], endianness),
            extra_params => FlirExtraParams,
        }

        Ok(Some(FlirCameraParams {
            temperature_params,
            camera_info,
            lens_info,
            filter_info,
            extra_params,
        }))
    }
}

/// Flir Camera Parameters
#[derive(Debug)]
pub struct FlirCameraParams {
    pub temperature_params: FlirTemperatureParams,
    pub camera_info: FlirCameraInfo,
    pub lens_info: FlirLensInfo,
    pub filter_info: FlirFilterInfo,
    pub extra_params: FlirExtraParams,
}

declare_parseable_structs! {
    /// Flir Temperature Parameters
    #[derive(Debug)]
    pub struct FlirTemperatureParams {
        pub emissivity => f32,
        pub object_distance => f32,

        pub reflected_apparent_temperature => f32,
        pub atmospheric_temperature => f32,
        pub ir_window_temperature => f32,
        pub ir_window_transmission => f32,

        _dummy_ignore => u32,

        pub relative_humidity => f32,
        _dummy_ignore_1 => [u32; 6],

        pub planck_r1 => f32,
        pub planck_b => f32,
        pub planck_f => f32,
        _dummy_ignore_2 => [u32; 3],

        pub atmospheric_transmission_alpha_1 => f32,
        pub atmospheric_transmission_alpha_2 => f32,
        pub atmospheric_transmission_beta_1 => f32,
        pub atmospheric_transmission_beta_2 => f32,
        pub atmospheric_transmission_x => f32,
        _dummy_ignore_3 => [u32; 3],

        pub camera_temperature_range => [f32; 8],
    }

    /// Flir Camera Info
    #[derive(Debug)]
    pub struct FlirCameraInfo {
        pub camera_mode => [u8; 32],
        pub camera_part_number => [u8; 16],
        pub camera_serial_number => [u8; 16],
        pub camera_software => [u8; 16],
    }

    /// Flir Lens Info
    #[derive(Debug)]
    pub struct FlirLensInfo {
        pub lens_mode => [u8; 32],
        pub lens_part_number => [u8; 16],
        pub lens_serial_number => [u8; 16],
    }

    /// Flir Filter Info
    #[derive(Debug)]
    pub struct FlirFilterInfo {
        pub filter_mode => [u8; 32],
        pub filter_part_number => [u8; 16],
        pub filter_serial_number => [u8; 16],
    }

    /// Flir Extra Info
    #[derive(Debug)]
    pub struct FlirExtraParams {
        pub planck_o => i32,
        pub planck_r2 => f32,
        pub raw_value_ranges => [u16; 4],
    }
}
