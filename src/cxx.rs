use crate::geometry::volume::Volume;
use crate::utils::error::ctrlc_catched;


#[cxx::bridge]
pub mod ffi {
    // ===========================================================================================
    //
    // Errors interface.
    //
    // ===========================================================================================

    struct Error {
        tp: ErrorType,
        message: String,
    }

    #[repr(i32)]
    enum ErrorType {
        None,
        FileNotFoundError,
        Geant4Exception,
        KeyboardInterrupt,
        MemoryError,
        ValueError,
    }

    // ===========================================================================================
    //
    // Geometry interface.
    //
    // ===========================================================================================

    struct BoxShape {
        size: [f64; 3],
    }

    struct CylinderShape {
        radius: f64,
        length: f64,
        thickness: f64,
    }

    #[repr(i32)]
    enum ShapeType {
        Box,
        Cylinder,
        Sphere,
        Tessellation,
    }

    struct SphereShape {
        radius: f64,
    }

    struct TessellatedShape {
        facets: Vec<f32>,
    }

    // ===========================================================================================
    //
    // Materials interface.
    //
    // ===========================================================================================

    struct Element {
        name: String,
        symbol: String,
        Z: f64,
        A: f64,
    }

    #[repr(u32)]
    enum G4State {
      kStateUndefined = 0,
      kStateSolid,
      kStateLiquid,
      kStateGas
    }

    struct MaterialProperties {
        name: String,
        density: f64,
        state: G4State,
    }

    #[derive(Hash)]
    struct Mixture {
        properties: MaterialProperties,
        components: Vec<MixtureComponent>,
    }

    #[derive(PartialEq, PartialOrd)]
    struct MixtureComponent {
        name: String,
        weight: f64,
    }

    #[derive(Hash)]
    struct Molecule {
        properties: MaterialProperties,
        components: Vec<MoleculeComponent>,
    }

    #[derive(Hash, PartialEq, PartialOrd)]
    struct MoleculeComponent {
        name: String,
        weight: u32,
    }

    // ===========================================================================================
    //
    // Units interface.
    //
    // ===========================================================================================

    #[derive(Debug)]
    struct UnitDefinition {
        name: String,
        symbol: String,
        value: f64,
    }

    // ===========================================================================================
    //
    // c++ imports.
    //
    // ===========================================================================================

    unsafe extern "C++" {
        include!("calzone.h");

        // Errors interface.
        fn initialise_errors();
        fn get_error() -> SharedPtr<Error>;

        // Geometry interface.
        type GeometryBorrow;
        fn create_geometry(volume: Box<Volume>) -> SharedPtr<GeometryBorrow>;

        fn check(self: &GeometryBorrow, resolution: i32) -> SharedPtr<Error>;
        fn compute_box(self: &GeometryBorrow, volume: &str, frame: &str) -> [f64; 6];
        fn dump(self: &GeometryBorrow, path: &str) -> SharedPtr<Error>;
        fn set_goupil(self: &GeometryBorrow);

        // Material interface.
        type G4State;
        fn add_element(element: &Element) -> SharedPtr<Error>;
        fn add_mixture(element: &Mixture) -> SharedPtr<Error>;
        fn add_molecule(element: &Molecule) -> SharedPtr<Error>;

        // Units interface.
        fn export_units(units: &mut Vec<UnitDefinition>);
    }

    // ===========================================================================================
    //
    // Rust exports.
    //
    // ===========================================================================================

    extern "Rust" {
        // Errors interface.
        fn ctrlc_catched() -> bool;

        // Geometry interface.
        type Volume;

        fn box_shape(self: &Volume) -> &BoxShape;
        fn cylinder_shape(self: &Volume) -> &CylinderShape;
        fn is_rotated(self: &Volume) -> bool;
        fn material(self: &Volume) -> &String;
        fn name(self: &Volume) -> &String;
        fn overlaps(self: &Volume) -> &[[String; 2]];
        fn position(self: &Volume) -> [f64; 3];
        fn rotation(self: &Volume) -> &[[f64; 3]];
        fn shape(self: &Volume) -> ShapeType;
        fn sphere_shape(self: &Volume) -> &SphereShape;
        fn tessellated_shape(self: &Volume) -> &TessellatedShape;
        fn volumes(self: &Volume) -> &[Volume];

        // Materials interface.
        fn get_hash(self: &Mixture) -> u64;
        fn get_hash(self: &Molecule) -> u64;
    }
}
