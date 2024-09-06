mod args;
mod proc;

use anyhow::Result;
use thermal::cli::process_paths_par;

use crate::{
    args::Args,
    proc::{copy_exif_and_xmp, transform_image_tiff, TransformArgs},
};

fn main() -> Result<()> {
    let args = Args::from_cmd_line()?;
    let t_args = TransformArgs::from_args(&args);
    let Args {
        paths,
        is_json,
        copy_exif,
        ..
    } = args;

    use rayon::prelude::*;
    let count = process_paths_par(paths, is_json)
        .into_par_iter()
        .map(|p| -> Result<()> {
            let inp = p?;
            let out_path = transform_image_tiff(&inp, &t_args)?;
            if copy_exif {
                copy_exif_and_xmp(&inp.filename, &out_path)?;
            }
            Ok(())
        })
        .try_fold(
            || 0usize,
            |acc, res| -> Result<_> {
                res?;
                Ok(acc + 1)
            },
        )
        .try_reduce(|| 0, |a, b| Ok(a + b))?;

    eprintln!("Processed {} images", count);
    eprintln!(
        "Transform equation: V = {} + {} C",
        t_args.coeffs[0], t_args.coeffs[1]
    );
    eprintln!(
        "Inverse equation: C = {} + {} V",
        -t_args.coeffs[0] / t_args.coeffs[1],
        1. / t_args.coeffs[1]
    );
    Ok(())
}
