use crate::GEANT4_VERSION;
use crate::utils::error::{ctrlc_catched, Error};
use crate::utils::error::ErrorKind::{KeyboardInterrupt, ValueError};
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use pyo3::prelude::*;
use regex::Regex;
use reqwest::StatusCode;
use std::borrow::Cow;
use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use temp_dir::TempDir;


/// Download Geant4 data.
#[pyfunction]
#[pyo3(signature=(destination=None, *, verbose=None))]
pub fn download(destination: Option<&str>, verbose: Option<bool>) -> PyResult<()> {
    const BASE_URL: &str = "https://geant4.web.cern.ch/download";
    const NAMES: [&str; 12] = [
        "G4ABLA", "G4CHANNELING", "G4EMLOW", "G4ENSDFSTATE", "G4INCL", "G4NDL", "G4PARTICLEXS",
        "G4PII", "G4SAIDDATA", "PhotonEvaporation", "RadioactiveDecay", "RealSurface",
    ];

    let url = format!("{}/{}.html", BASE_URL, GEANT4_VERSION);
    let response = reqwest::blocking::get(&url)
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
    let content = response.text().unwrap();

    let mut datasets = Vec::new();
    for name in NAMES {
        let pattern = format!("{}[.]([0-9][.][0-9])([.][0-9])?[.]tar[.]gz", name);
        let re = Regex::new(&pattern).unwrap();
        if let Some(captures) = re.captures(&content) {
            let version = captures.get(1).unwrap().as_str();
            let patch = captures.get(2).map(|patch| patch.as_str());
            datasets.push(DataSet { name, version, patch })
        }
    }

    let destination = match destination {
        None => Cow::Owned(default_path()),
        Some(destination) => Cow::Borrowed(Path::new(destination)),
    };

    let verbose = verbose.unwrap_or(false);

    std::fs::create_dir_all(&destination)?;
    for dataset in datasets {
        dataset.download(&destination, verbose)?;
    }

    Ok(())
}

pub fn default_path() -> PathBuf {
    if cfg!(windows) {
        let appdata = env::var("LOCALAPPDATA").unwrap();
        Path::new(&appdata)
            .join("calzone/data")
    } else {
        let home = env::var("HOME").unwrap();
        Path::new(&home)
            .join(".local/share/calzone/data")
    }
}

struct DataSet<'a> {
    name: &'a str,
    version: &'a str,
    patch: Option<&'a str>,
}

impl<'a> DataSet<'a> {
    fn dirname(&self) -> String {
        format!("{}{}", self.name, self.version)
    }

    fn download(&self, destination: &Path, verbose: bool) -> PyResult<()> {
        // Download tarball.
        const BASE_URL: &str = "https://cern.ch/geant4-data/datasets";
        let tarname = self.tarname();
        let url = format!("{}/{}", BASE_URL, tarname);
        let mut response = reqwest::blocking::get(&url)
            .map_err(|err| {
                let why = format!("{}: {}", self.name, err);
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
            let why = format!("{}: {}", self.name, reason);
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
                "extracting {} to {}/{}",
                tarname,
                destination.display(),
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
        let patch = match self.patch {
            None => "".to_string(),
            Some(patch) => format!(".{}", patch),
        };
        let name = if self.name.starts_with("G4") {
            Cow::Borrowed(self.name)
        } else {
            Cow::Owned(format!("G4{}", self.name))
        };
        format!("{}.{}{}.tar.gz", name, self.version, patch)
    }
}
