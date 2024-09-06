use std::path::PathBuf;
use std::{env, fs};

use anyhow::Result;
use criterion::*;
use glob::{glob_with, MatchOptions};
use img_parts::jpeg::Jpeg;
use thermal::{dji::RJpeg, ThermalImage};

pub struct Samples<T>(Vec<T>);
impl<T> Samples<T> {
    pub fn sampler<'a>(&'a self) -> impl FnMut() -> &'a T {
        let mut curr = 0;
        move || {
            let ret = curr;
            curr += 1;
            curr %= self.0.len();
            &self.0[ret]
        }
    }
    pub fn from_fn<F: FnMut() -> T>(size: usize, mut proc: F) -> Self {
        Self((0..size).map(|_| proc()).collect())
    }
}

fn get_samples(key: &'static str) -> Result<Vec<PathBuf>> {
    let base = env::var(key)?;
    let mut opts = MatchOptions::new();
    opts.case_sensitive = false;
    let samples: Vec<_> = glob_with(&format!("{base}/**/*.jpg"), opts)?
        .take(5)
        .map(|r| Result::Ok(r?))
        .collect::<Result<_>>()?;
    Ok(samples)
}

fn temperature(c: &mut Criterion) {
    c.bench_function("flir_parse", |b| {
        let samples = get_samples("FLIR_SAMPLES").expect("samples");
        b.iter(|| {
            for path in samples.iter() {
                ThermalImage::try_from_rjpeg_path(path).unwrap();
            }
        })
    });

    c.bench_function("dji_parse", |b| {
        let samples = get_samples("DJI_SAMPLES").expect("samples");
        b.iter(|| {
            for path in samples.iter() {
                RJpeg::try_from_path(path).unwrap();
            }
        })
    });

    c.bench_function("jpeg_parse", |b| {
        let samples = get_samples("FLIR_SAMPLES").expect("samples");
        b.iter(|| {
            for path in samples.iter() {
                let _ = Jpeg::from_bytes(fs::read(path).unwrap().into());
            }
        })
    });
}

criterion_group! {
    name = parsing;
    config = Criterion::default().sample_size(10);
    targets = temperature
}

criterion_main!(parsing);
