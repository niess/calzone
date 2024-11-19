use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;


fn main() {
    let command = Command::new("geant4-config")
        .arg("--prefix")
        .output();
    let geant4_prefix = match command {
        Ok(output) => {
            let geant4_prefix = String::from_utf8(output.stdout)
                .expect("could not parse Geant4 prefix")
                .trim()
                .to_string();
            export_geant4_version("geant4-config");
            geant4_prefix
        },
        Err(_) => {
            let prefix = "geant4";
            if !Path::new(prefix).is_dir() {
                panic!("could not locate Geant4");
            }
            let geant4_config = format!("{prefix}/bin/geant4-config");
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

    cxx_build::bridge("src/cxx.rs")
        .std("c++17")
        .define("FMT_HEADER_ONLY", "")
        .include(&fmt_include)
        .include(&geant4_include)
        .include(&goupil_include)
        .include("src")
        .files(sources)
        .define("G4GOUPIL_INITIALISE", "g4goupil_initialise")
        .file(&goupil_source)
        .compile("geant4");

    println!("cargo:rerun-if-changed=src/cxx.rs");

    for path in sources {
        println!("cargo:rerun-if-changed={}", path);
    }

    for path in headers {
        println!("cargo:rerun-if-changed={}", path);
    }

    println!("cargo:rustc-link-search={}", geant4_lib.display());
    println!("cargo:rustc-link-lib=G4gdml");
    println!("cargo:rustc-link-lib=G4physicslists");
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
        .expect("could not fetch Geant4 version");
    let geant4_version = String::from_utf8(output.stdout)
        .expect("could not parse Geant4 version")
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
