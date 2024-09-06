use anyhow::{bail, Error, Result};
use ndarray::Array2;
use std::{
    convert::TryFrom,
    mem::{size_of, MaybeUninit},
    path::Path,
};

use dji_thermal_sys::*;

#[derive(Debug)]
pub struct RJpeg {
    handle: DIRP_HANDLE,
}

unsafe impl Send for RJpeg {}

impl RJpeg {
    pub fn try_from_path(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::try_from_bytes(data)
    }

    pub fn try_from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let size = bytes.len() as i32;
        let mut handle = MaybeUninit::uninit();
        let ret = unsafe { dirp_create_from_rjpeg(bytes.as_ptr(), size, handle.as_mut_ptr()) };
        if ret != 0 {
            bail!("could not parse rjpeg!");
        }

        Ok(RJpeg {
            handle: unsafe { handle.assume_init() },
        })
    }

    pub fn measurement_params(&self) -> Result<MeasurementParams> {
        let mut params = MaybeUninit::uninit();
        let ret = unsafe { dirp_get_measurement_params(self.handle, params.as_mut_ptr()) };
        if ret != 0 {
            bail!("could not read measurement params!");
        }

        Ok(unsafe { params.assume_init() })
    }

    pub fn dimensions(&self) -> Result<(i32, i32)> {
        let mut resolution = MaybeUninit::uninit();
        let ret = unsafe { dirp_get_rjpeg_resolution(self.handle, resolution.as_mut_ptr()) };
        if ret != 0 {
            bail!("could not rjpeg dimensions!");
        }

        let resolution = unsafe { resolution.assume_init() };
        Ok((resolution.width, resolution.height))
    }

    pub fn temperatures(&self) -> Result<Array2<f32>> {
        let (width, height) = self.dimensions()?;
        let num_values = width * height;

        let mut values = Vec::with_capacity(num_values as usize);
        let ret = unsafe {
            dirp_measure_ex(
                self.handle,
                values.as_mut_ptr(),
                num_values * size_of::<f32>() as i32,
            )
        };
        if ret != 0 {
            bail!("could not calculate rjpeg temperatures!");
        }
        unsafe {
            values.set_len(num_values as usize);
        }

        let values = Array2::from_shape_vec((height as usize, width as usize), values)?;
        Ok(values)
    }
}

pub use dji_thermal_sys::dirp_measurement_params_t as MeasurementParams;

impl TryFrom<Vec<u8>> for RJpeg {
    type Error = Error;

    fn try_from(value: Vec<u8>) -> Result<Self> {
        RJpeg::try_from_bytes(value)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{Context, Result};
    use glob::{glob_with, MatchOptions};

    use std::{env, path::Path};

    use super::RJpeg;
    use crate::{stats::Stats, ThermalImage};

    #[test]
    fn compare_temperatures() -> Result<()> {
        let base = env::var("THERMAL_DATASETS_PATH").context("env `THERMAL_DATASETS_PATH`")?;
        let mut opts = MatchOptions::new();
        opts.case_sensitive = false;
        eprintln!("Verifying {base}/**/*.jpg");
        for path in glob_with(&format!("{base}/**/*.jpg"), opts)? {
            let path = path?;
            eprintln!("Reading {path}...", path = path.display());
            match compare_at_path(&path) {
                Ok(stats) => {
                    eprintln!("\t{stats:?}");
                }
                Err(e) => {
                    eprintln!("\t{e:#}");
                }
            }
        }
        Ok(())
    }

    fn compare_at_path(path: &Path) -> Result<Stats> {
        let rj = RJpeg::try_from_path(path).context("dji rjpeg parsing failed")?;
        eprintln!("\topened successfully.");

        let (wid, ht) = rj.dimensions()?;
        let wid = wid as usize;
        let ht = ht as usize;

        eprintln!("\tdims: {wid}x{ht}");

        let params = rj.measurement_params()?;
        eprintln!("\tparams: {params:?}");

        let t_dji = rj.temperatures()?;

        let thermal = ThermalImage::try_from_rjpeg_path(path).context("flir parsing failed")?;
        let temp_t = thermal
            .settings
            .temperature_transform(params.distance as f64);
        assert_eq!(thermal.image.dim(), (ht, wid));

        let mut stats = Stats::default();
        for row in 0..ht {
            for col in 0..wid {
                let raw = thermal.image[(row, col)];
                let temp = temp_t(raw);
                stats += (temp - t_dji[(row, col)] as f64).abs();
            }
        }
        Ok(stats)
    }
}
