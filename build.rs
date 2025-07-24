use flate2::read::GzDecoder;
use reqwest::StatusCode;
use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Archive;
use temp_dir::TempDir;


const LINESEP: &str = if cfg!(windows) { "\r\n" } else { "\n" };

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
            let prefix = "geant4";
            if !Path::new(prefix).is_dir() {
                if let Err(msg) = download_geant4() {
                    panic!("{}", boxed(&msg))
                }
            }
            const SEP: char = std::path::MAIN_SEPARATOR;
            let geant4_config = format!("{prefix}{SEP}bin{SEP}{GEANT4_CONFIG}");
            export_geant4_version(&geant4_config);
            prefix.to_string()
        },
    };
    let geant4_include = make_path(&geant4_prefix, &["include/Geant4"]);
    let geant4_lib = make_path(&geant4_prefix, &["lib", "lib64"]);

    let goupil_prefix = "deps/goupil/src";
    let goupil_include = make_path(&goupil_prefix, &["interfaces/geant4"]);
    let goupil_source = make_path(&goupil_prefix, &["interfaces/geant4/G4Goupil.cc"]);

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
        .include("src")
        .files(sources)
        .define("G4GOUPIL_INITIALISE", "g4goupil_initialise")
        .file(&goupil_source);

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
        .join("geant4_version.rs");
    std::fs::write(
        &path,
        format!(
            "const GEANT4_VERSION: &str = \"{}\";",
            geant4_version,
        ),
    ).unwrap();
}

fn download_geant4() -> Result<(), String> {
    const GEANT4_VERSION: &str = "11.3.0";
    const BASE_URL: &str = "https://github.com/niess/calzone/releases/download/";

    const TAG: &str =
        if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            "win_amd64"
        } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            "macosx_11_0_arm64"
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            "macosx_11_0_x86_64"
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            "manylinux2014_aarch64"
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            "manylinux2014_x86_64"
        } else {
            panic!("unsupported os/arch")
        };
    let tarname = format!("geant4-{}-{}.tar.gz", GEANT4_VERSION, TAG);
    let url = format!("{}/geant4-{}/{}", BASE_URL, GEANT4_VERSION, tarname);

    let mut response = reqwest::blocking::get(&url)
        .map_err(|err| format!("could not GET/ {} ({err})", url))?;
    if response.status() != StatusCode::OK {
        let status = response.status();
        let reason = status
            .canonical_reason()
            .unwrap_or_else(|| status.as_str());
        return Err(format!("failed to GET/ {} ({})", url, reason))
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
    archive.unpack(".")
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
