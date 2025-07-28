use crate::utils::error::{ctrlc_catched, Error};
use crate::utils::error::ErrorKind::{KeyboardInterrupt, ValueError};
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use pyo3::prelude::*;
use reqwest::StatusCode;
use std::borrow::Cow;
use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use temp_dir::TempDir;


// Fetch the Geant4 datasets (exported by the build script).
const DATASETS: &[DataSet] = include!(concat!(env!("OUT_DIR"), "/geant4_datasets.in"));


/// Download Geant4 data.
#[pyfunction]
#[pyo3(signature=(destination=None, *, exclude=None, force=false, include=None, verbose=false))]
pub fn download(
    destination: Option<&str>,
    exclude: Option<Vec<String>>,
    force: Option<bool>,
    include: Option<Vec<String>>,
    verbose: Option<bool>,
    ) -> PyResult<()> {
    let destination = match destination {
        None => Cow::Owned(default_path()),
        Some(destination) => Cow::Borrowed(Path::new(destination)),
    };

    let force = force.unwrap_or(false);
    let verbose = verbose.unwrap_or(false);

    let datasets = DATASETS
        .iter()
        .filter(|dataset| match include.as_ref() {
            Some(include) => include.iter().any(|include| dataset.name == include),
            None => true,
        })
        .filter(|dataset| match exclude.as_ref() {
            Some(exclude) => exclude.iter().all(|exclude| dataset.name != exclude),
            None => true,
        });

    std::fs::create_dir_all(&destination)?;
    for dataset in datasets {
        dataset.download(&destination, force, verbose)?;
    }

    Ok(())
}

pub fn default_path() -> PathBuf {
    if cfg!(windows) {
        let appdata = env::var("LOCALAPPDATA").unwrap();
        Path::new(&appdata)
            .join("calzone\\data")
    } else {
        let home = env::var("HOME").unwrap();
        Path::new(&home)
            .join(".local/share/calzone/data")
    }
}

struct DataSet<'a> {
    name: &'a str,
    version: &'a str,
}

impl<'a> DataSet<'a> {
    fn dirname(&self) -> String {
        format!("{}{}", self.name, self.version)
    }

    fn download(&self, destination: &Path, force: bool, verbose: bool) -> PyResult<()> {
        if !force && destination.join(&self.dirname()).exists() {
            return Ok(())
        }

        // Download tarball.
        const BASE_URL: &str = "https://cern.ch/geant4-data/datasets";
        let tarname = self.tarname();
        let url = format!("{}/{}", BASE_URL, tarname);
        let mut response = reqwest::blocking::get(&url)
            .map_err(|err| {
                let why = format!("{}: {}", url, err);
                Error::new(ValueError)
                    .what("download")
                    .why(&why)
                    .to_err()
            })?;
        if response.status() != StatusCode::OK {
            let status = response.status();
            let reason = status
                .canonical_reason()
                .unwrap_or_else(|| status.as_str());
            let why = format!("{}: {}", url, reason);
            let err = Error::new(ValueError)
                .what("download")
                .why(&why)
                .to_err();
            return Err(err);
        }

        let length = response.headers()["content-length"]
            .to_str()
            .unwrap()
            .parse::<usize>()
            .unwrap();

        let bar = if verbose {
            println!(
                "downloading {} from {}",
                tarname,
                BASE_URL,
            );
            let bar = ProgressBar::new(length as u64);
            let style = ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar:.green}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
            )
                .unwrap()
                .progress_chars("█▉▊▋▌▍▎▏  ");
            bar.set_style(style);
            Some(bar)
        } else {
            None
        };

        let tmpdir = TempDir::new()?;
        let tarpath = tmpdir.child(&tarname);
        let mut tarfile = std::fs::File::create(&tarpath)?;
        const CHUNK_SIZE: usize = 2048;
        let mut buffer = [0_u8; CHUNK_SIZE];
        let mut size = 0_usize;
        loop {
            let n = response.read(&mut buffer)?;
            if n > 0 {
                tarfile.write(&buffer[0..n])?;
                size += n;
                if size >= length { break }
                if let Some(bar) = &bar {
                    bar.set_position(size as u64);
                }
            }
            if ctrlc_catched() {
                if let Some(bar) = &bar {
                    bar.finish_and_clear();
                }
                let err = Error::new(KeyboardInterrupt)
                    .what(self.name);
                return Err(err.to_err())
            }
        }
        drop(tarfile);
        if let Some(bar) = &bar {
            bar.finish_and_clear();
        }

        // Extract data.
        if verbose {
            println!(
                "extracting {} to {}{}{}",
                tarname,
                destination.display(),
                std::path::MAIN_SEPARATOR,
                self.dirname(),
            );
        }

        let tarfile = std::fs::File::open(&tarpath)?;
        let tar = GzDecoder::new(tarfile);
        let mut archive = Archive::new(tar);
        archive.unpack(destination)?;

        Ok(())
    }

    fn tarname(&self) -> String {
        let name = if self.name.starts_with("G4") {
            Cow::Borrowed(self.name)
        } else {
            Cow::Owned(format!("G4{}", self.name))
        };
        format!("{}.{}.tar.gz", name, self.version)
    }
}
