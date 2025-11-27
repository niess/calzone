use flate2::read::GzDecoder;
use regex::Regex;
use reqwest::StatusCode;
use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Archive;
use temp_dir::TempDir;


const LINESEP: &str = if cfg!(windows) { "\r\n" } else { "\n" };

const MISSING_GEANT4: &str = "Could not locate **Geant4** install";

fn main() {
    const GEANT4_CONFIG: &str = if cfg!(windows) {
        "geant4-config.cmd"
    } else {
        "geant4-config"
    };
    let command = Command::new(GEANT4_CONFIG)
        .arg("--prefix")
        .output();
    let geant4_prefix = match command {
        Ok(output) => {
            let geant4_prefix = String::from_utf8(output.stdout)
                .expect(&boxed("Could not parse **Geant4** prefix"))
                .trim()
                .to_string();
            export_geant4_version(GEANT4_CONFIG);
            geant4_prefix
        },
        Err(_) => {
            let geant4_prefix = env::var_os("GEANT4_PREFIX")
                .expect(&boxed(MISSING_GEANT4));
            let path = Path::new(&geant4_prefix);
            if !path.is_dir() {
                if let Err(msg) = download_geant4(&path) {
                    panic!("{}", boxed(&msg))
                }
            }
            let geant4_prefix = geant4_prefix.to_string_lossy().to_string();
            const SEP: char = std::path::MAIN_SEPARATOR;
            let geant4_config = format!("{geant4_prefix}{SEP}bin{SEP}{GEANT4_CONFIG}");
            export_geant4_version(&geant4_config);
            geant4_prefix
        },
    };
    export_geant4_datasets(&geant4_prefix);

    let geant4_include = make_path(&geant4_prefix, &["include/Geant4"]);
    let geant4_lib = make_path(&geant4_prefix, &["lib", "lib64"]);

    let goupil_prefix = "deps/goupil/src";
    let goupil_include = make_path(&goupil_prefix, &["interfaces/geant4"]);
    let goupil_source = make_path(&goupil_prefix, &["interfaces/geant4/G4Goupil.cc"]);

    let mulder_prefix = "deps/mulder/src";
    let mulder_include = make_path(&mulder_prefix, &["module/interface/geant4"]);
    let mulder_source = make_path(&mulder_prefix, &["module/interface/geant4/G4Mulder.cc"]);

    let fmt_prefix = "deps/fmt";
    let fmt_include = make_path(&fmt_prefix, &["include"]);

    let sources = [
        "src/geometry.cc",
        "src/geometry/materials.cc",
        "src/geometry/solids.cc",
        "src/geometry/mesh.cc",
        "src/simulation.cc",
        "src/simulation/geometry.cc",
        "src/simulation/physics.cc",
        "src/simulation/random.cc",
        "src/simulation/sampler.cc",
        "src/simulation/source.cc",
        "src/simulation/tracker.cc",
        "src/utils/convert.cc",
        "src/utils/error.cc",
        "src/utils/os.cc",
        "src/utils/units.cc",
    ];

    let headers = [
        "src/calzone.h",
        "src/geometry/solids.h",
        "src/geometry/mesh.h",
        "src/simulation/geometry.h",
        "src/simulation/physics.h",
        "src/simulation/random.h",
        "src/simulation/sampler.h",
        "src/simulation/source.h",
        "src/simulation/tracker.h",
    ];

    let mut bridge = cxx_build::bridge("src/cxx.rs");
    bridge
        .std("c++17")
        .define("FMT_HEADER_ONLY", "")
        .include(&fmt_include)
        .include(&geant4_include)
        .include(&goupil_include)
        .include(&mulder_include)
        .include("src")
        .files(sources)
        .define("G4GOUPIL_INITIALISE", "g4goupil_initialise")
        .define("G4MULDER_INITIALISE", "g4mulder_initialise")
        .file(&goupil_source)
        .file(&mulder_source);

    #[cfg(windows)]
    bridge
        .define("_CONSOLE", "")
        .define("_WIN32", "")
        .define("WIN32", "")
        .define("DOS", "")
        .define("XPNET", "")
        .define("_CRT_SECURE_NO_DEPRECATE", "")
        .flags(["-GR", "-EHsc", "-Zm200", "-nologo"]);

    bridge.compile("geant4");

    println!("cargo:rerun-if-changed=src/cxx.rs");

    for path in sources {
        println!("cargo:rerun-if-changed={}", path);
    }

    for path in headers {
        println!("cargo:rerun-if-changed={}", path);
    }

    println!("cargo:rustc-link-search={}", geant4_lib.display());
    const LIBS: [&str; 17] = [
        "G4analysis", "G4physicslists", "G4run", "G4event", "G4tracking", "G4processes",
        "G4digits_hits", "G4track", "G4particles", "G4geometry", "G4materials",
        "G4graphics_reps", "G4intercoms", "G4global", "G4clhep", "G4ptl", "G4zlib"
    ];
    for lib in LIBS {
        println!("cargo:rustc-link-lib={}", lib);
    }
}

fn make_path(prefix: &str, locations: &[&str]) -> PathBuf {
    for location in locations {
        let path = Path::new(prefix).join(location);
        if path.exists() {
            return path;
        }
    }
    let path = Path::new(prefix).join(locations[0]);
    panic!("missing {}", path.display())
}

fn export_geant4_version(geant4_config: &str) {
    let output = Command::new(geant4_config)
        .arg("--version")
        .output()
        .expect(&boxed("could not fetch Geant4 version"));
    let geant4_version = String::from_utf8(output.stdout)
        .expect(&boxed("could not parse Geant4 version"))
        .trim()
        .to_string();

    let out_dir = env::var_os("OUT_DIR")
        .unwrap();
    let path = Path::new(&out_dir)
        .join("geant4_version.in");
    std::fs::write(&path, format!("\"{}\"", geant4_version)).unwrap();
}

fn export_geant4_datasets(geant4_prefix: &str) {
    let path = Path::new(geant4_prefix).join("bin").join("geant4.sh");
    let content = std::fs::read_to_string(path)
        .expect(&boxed("could not read $GEANT4_PREFIX/bin/geant4.sh"));

    let re = Regex::new("# export G4[A-Z]+=[$]GEANT4_DATA_DIR/(G?4?[a-zA-Z]+)([0-9.]+)").unwrap();
    let mut lines = Vec::new();
    lines.push("&[".to_owned());
    for captures in re.captures_iter(&content) {
        let line = format!(
            "    DataSet {{ name: \"{}\", version: \"{}\" }},",
            captures.get(1).unwrap().as_str(),
            captures.get(2).unwrap().as_str(),
        );
        lines.push(line);
    }
    lines.push("]".to_owned());
    let lines = lines.join(LINESEP);

    let out_dir = env::var_os("OUT_DIR")
        .unwrap();
    let path = Path::new(&out_dir)
        .join("geant4_datasets.in");
    std::fs::write(&path, lines).unwrap();
}

fn download_geant4(geant4_prefix: &Path) -> Result<(), String> {
    const BASE_URL: &str = "https://github.com/niess/calzone/releases/download";

    let dirname = geant4_prefix.file_name()
        .and_then(|dirname| dirname.to_str())
        .ok_or_else(|| MISSING_GEANT4.to_owned())?;
    let items: Vec<_> = dirname.split('-').collect();
    if (items.len() != 3) || (items[0] != "geant4") {
        return Err(MISSING_GEANT4.to_owned())
    }
    let (version, tag) = (items[1], items[2]);

    let tarname = format!("geant4-{}-{}.tar.gz", version, tag);
    let url = format!("{}/geant4-{}/{}", BASE_URL, version, tarname);

    let mut response = reqwest::blocking::get(&url)
        .map_err(|err| format!("Could not GET/ {} ({err})", url))?;
    if response.status() != StatusCode::OK {
        let status = response.status();
        let reason = status
            .canonical_reason()
            .unwrap_or_else(|| status.as_str());
        return Err(format!("Failed to GET/ {} ({})", url, reason))
    }

    let length = response.headers()["content-length"]
        .to_str()
        .unwrap()
        .parse::<usize>()
        .unwrap();

    let tmpdir = TempDir::new()
        .map_err(|err| format!("{}", err))?;
    let tarpath = tmpdir.child(&tarname);
    let mut tarfile = std::fs::File::create(&tarpath)
        .map_err(|err| format!("{}", err))?;
    const CHUNK_SIZE: usize = 2048;
    let mut buffer = [0_u8; CHUNK_SIZE];
    let mut size = 0_usize;
    loop {
        let n = response.read(&mut buffer)
            .map_err(|err| format!("{}", err))?;
        if n > 0 {
            tarfile.write(&buffer[0..n])
                .map_err(|err| format!("{}", err))?;
            size += n;
            if size >= length { break }
        }
    }
    drop(tarfile);

    let tarfile = std::fs::File::open(&tarpath)
        .map_err(|err| format!("{}", err))?;
    let tar = GzDecoder::new(tarfile);
    let mut archive = Archive::new(tar);
    let destination = geant4_prefix
        .parent()
        .filter(|destination| destination != &Path::new(""))
        .unwrap_or_else(|| Path::new("."));
    archive.unpack(&destination)
        .map_err(|err| format!("{}", err))?;

    Ok(())
}

fn boxed(text: &str) -> String {
    let n = text.len() + 6;
    let ruler = format!("{:=^width$}", "", width = n + 4);
    let blank = format!("=={:^width$}==", "", width = n);
    let text = format!("=={:^width$}==", text, width = n);
    vec![
        "".to_owned(), ruler.clone(), blank.clone(), text, blank, ruler, "".to_owned(),
        "".to_owned()
    ].join(LINESEP)
}
