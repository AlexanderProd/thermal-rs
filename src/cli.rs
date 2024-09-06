//! Helpers to parse CLI arguments in the accompanying
//! binaries.
//!
//! APIs here shouldn't be considered stable / used as a
//! library.

use std::{
    convert::{TryFrom, TryInto},
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use anyhow::{Context, Error, Result};
pub use clap::{App, Arg};
use indicatif::{ProgressBar, ProgressStyle};
pub use inflector::Inflector;
use rayon::iter::{once, Either, IntoParallelIterator, ParallelIterator};
use serde_derive::*;
use serde_json::Deserializer;

use crate::{dji::RJpeg, ThermalExiftoolJson, ThermalImage};

#[macro_export]
macro_rules! args_parser {
    ($name:expr) => {{
        $crate::cli::App::new($name)
            .version(clap::crate_version!())
            .author(clap::crate_authors!())
    }};
}

#[macro_export]
macro_rules! arg {
    ($name:expr) => {{
        use $crate::cli::Inflector;
        $crate::cli::Arg::with_name($name).value_name(&$name.to_screaming_snake_case())
    }};
}

#[macro_export]
macro_rules! opt {
    ($name:expr) => {{
        use $crate::cli::Inflector;
        $crate::cli::Arg::with_name($name)
            .long(&$name.to_kebab_case())
            .value_name(&$name.to_screaming_snake_case())
    }};
}

pub type GenericImage = Either<ThermalImage, RJpeg>;
pub struct ThermalInput {
    pub filename: String,
    pub image: GenericImage,
}

#[allow(dead_code)]
impl ThermalInput {
    fn try_from_image_path(filename: String) -> Result<Self> {
        let image = ThermalImage::try_from_rjpeg_path(&filename)
            .map(Either::Left)
            .or_else::<Error, _>(|_| Ok(Either::Right(RJpeg::try_from_path(Path::new(&filename))?)))
            .context("could not parse thermal image: tried FLIR, DJI")?;
        Ok(ThermalInput { filename, image })
    }
    fn try_from_exiftool_json<R: Read>(rdr: R) -> Result<Vec<Result<Self>>> {
        Ok(serde_json::from_reader::<R, Vec<JsonFormat>>(rdr)?
            .into_iter()
            .map(|j| j.try_into())
            .collect())
    }
    fn stream_from_exiftool_json<R: Read>(rdr: R) -> impl Iterator<Item = Result<Self>> {
        Deserializer::from_reader(rdr)
            .into_iter::<JsonFormat>()
            .map(|j| -> Result<_> { j?.try_into() })
    }
}

#[derive(Deserialize)]
struct JsonFormat {
    #[serde(rename = "SourceFile")]
    pub filename: String,

    #[serde(flatten)]
    pub image: ThermalExiftoolJson,
}
impl TryFrom<JsonFormat> for ThermalInput {
    type Error = anyhow::Error;

    fn try_from(j: JsonFormat) -> Result<Self> {
        Ok(Self {
            filename: j.filename,
            image: Either::Left(j.image.try_into()?),
        })
    }
}

pub fn process_paths_par(
    paths: Vec<String>,
    is_json: bool,
) -> impl IntoParallelIterator<Item = Result<ThermalInput>> {
    let bar = ProgressBar::new(paths.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {wide_bar:cyan/blue} {pos:>7}/{len:7}"),
    );
    let bar_dup = bar.clone();

    paths
        .into_par_iter()
        .map(move |p| {
            if is_json {
                let vec = File::open(p)
                    .map_err(|e| e.into())
                    .and_then(|f| ThermalInput::try_from_exiftool_json(BufReader::new(f)));
                match vec {
                    Ok(vec) => {
                        if vec.len() > 1 {
                            bar.inc_length(vec.len() as u64 - 1);
                        }
                        Either::Left(vec.into_par_iter())
                    }
                    Err(e) => Either::Right(once(Err(e))),
                }
            } else {
                Either::Right(once(ThermalInput::try_from_image_path(p)))
            }
        })
        .flatten()
        .inspect(move |_| bar_dup.inc(1))
}
