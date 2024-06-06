use std::path::Path;
use std::process::Command;


fn main() {
    let geant4_prefix = {
        let command = Command::new("geant4-config")
            .arg("--prefix")
            .output()
            .expect("could not locate Geant4");
        String::from_utf8(command.stdout)
            .expect("could not parse Geant4 prefix")
            .trim()
            .to_string()
    };
    let geant4_include = Path::new(&geant4_prefix)
        .join("include/Geant4");
    let geant4_lib = Path::new(&geant4_prefix)
        .join("lib"); // XXX might be lib64 on some systems.
                      //
    let goupil_prefix = "deps/goupil/src"; // XXX relative to this file?
    let goupil_include = Path::new(&goupil_prefix)
        .join("interfaces/geant4");
    let goupil_source = Path::new(&goupil_prefix)
        .join("interfaces/geant4/G4Goupil.cc");

    let fmt_prefix = "deps/fmt"; // XXX relative to this file?
    let fmt_include = Path::new(&fmt_prefix)
        .join("include");

    let sources = [
        "src/geometry.cc",
        "src/materials.cc",
        "src/utils/error.cc",
        "src/utils/units.cc",
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
    println!("cargo:rerun-if-changed=src/calzone.h");

    for path in sources {
        println!("cargo:rerun-if-changed={}", path);
    }

    println!("cargo:rustc-link-search={}", geant4_lib.display());
    println!("cargo:rustc-link-lib=G4gdml");
    println!("cargo:rustc-link-lib=G4physicslists");
}
