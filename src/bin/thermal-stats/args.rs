use anyhow::Result;
use clap::value_t_or_exit;
use thermal::{arg, args_parser, opt};

pub struct Args {
    pub paths: Vec<String>,
    pub distance: f64,
    pub is_json: bool,
}

impl Args {
    pub fn from_cmd_line() -> Result<Args> {
        let matches = args_parser!("thermal-stats")
            .about("Compute temperature stats from image.")
            .arg(
                opt!("json")
                    .short("j")
                    .takes_value(false)
                    .help("Paths are jsons created using exiftool (default: paths are rjpegs)"),
            )
            .arg(
                opt!("distance")
                    .short("d")
                    .help("Distance to use for calculation.  Default is 1.0"),
            )
            .arg(
                arg!("paths")
                    .required(true)
                    .multiple(true)
                    .help("Image / json paths"),
            )
            .get_matches();

        let paths = matches
            .values_of("paths")
            .unwrap()
            .map(|f| f.into())
            .collect();
        let distance = matches
            .is_present("distance")
            .then(|| value_t_or_exit!(matches.value_of("distance"), f64))
            .unwrap_or(1.0);
        let is_json = matches.is_present("json");

        Ok(Args {
            paths,
            distance,
            is_json,
        })
    }
}
