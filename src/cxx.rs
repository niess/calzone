use crate::geometry::volume::Volume;
use crate::geometry::tessellation::{SortedTessels, sort_tessels};
use crate::simulation::RunAgent;
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
        IndexError,
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

    #[repr(u32)]
    enum EInside {
      kOutside,
      kSurface,
      kInside,
    }

    struct EnvelopeShape {
        shape: ShapeType,
        safety: f64,
    }

    #[repr(i32)]
    enum ShapeType {
        Box,
        Cylinder,
        Envelope,
        Sphere,
        Tessellation,
    }

    struct SphereShape {
        radius: f64,
    }

    struct TessellatedShape {
        facets: Vec<f32>,
    }

    struct VolumeInfo { // From Geant4.
        material: String,
        solid: String,
        sensitive: bool,
        mother: String,
        daughters: Vec<String>,
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
    // Physics interface.
    //
    // ===========================================================================================

    #[repr(u32)]
    pub enum EmPhysicsModel {
        Dna,
        Livermore,
        None,
        Option1,
        Option2,
        Option3,
        Option4,
        Penelope,
        Standard,
    }

    #[repr(u32)]
    pub enum HadPhysicsModel {
        FTFP_BERT,
        FTFP_BERT_HP,
        QGSP_BERT,
        QGSP_BERT_HP,
        QGSP_BIC,
        QGSP_BIC_HP,
        None,
    }

    #[derive(Clone, Copy)]
    struct Physics {
        default_cut: f64,
        em_model: EmPhysicsModel,
        had_model: HadPhysicsModel,
    }

    // ===========================================================================================
    //
    // Source interface.
    //
    // ===========================================================================================

    #[derive(Clone, Copy)]
    struct Primary {
        pid: i32,
        energy: f64,
        position: [f64; 3],
        direction: [f64; 3],
    }

    // ===========================================================================================
    //
    // Tracker interface.
    //
    // ===========================================================================================

    #[derive(Clone, Copy)]
    struct Track {
        event: usize,
        tid: i32,
        parent: i32,
        pid: i32,
        creator: [u8; 16],
    }

    #[derive(Clone, Copy)]
    struct Vertex {
        event: usize,
        tid: i32,
        energy: f64,
        position: [f64; 3],
        direction: [f64; 3],
        process: [u8; 16],
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
        type EInside;

        type GeometryBorrow;
        fn create_geometry(volume: &Box<Volume>) -> SharedPtr<GeometryBorrow>;

        fn check(self: &GeometryBorrow, resolution: i32) -> SharedPtr<Error>;
        fn compute_box(self: &GeometryBorrow, volume: &str, frame: &str) -> [f64; 6];
        fn compute_origin(self: &GeometryBorrow, volume: &str, frame: &str) -> [f64; 3];
        fn describe_volume(self: &GeometryBorrow, name: &str) -> VolumeInfo;
        fn dump(self: &GeometryBorrow, path: &str) -> SharedPtr<Error>;
        fn set_goupil(self: &GeometryBorrow);

        // Material interface.
        type G4State;
        fn add_element(element: &Element) -> SharedPtr<Error>;
        fn add_mixture(element: &Mixture) -> SharedPtr<Error>;
        fn add_molecule(element: &Molecule) -> SharedPtr<Error>;

        // Simulation interface.
        fn drop_simulation();
        fn run_simulation(agent: &mut RunAgent, verbose: bool) -> SharedPtr<Error>;

        type G4VPhysicalVolume;
        fn GetName(self: &G4VPhysicalVolume) -> &G4String;

        // Units interface.
        fn export_units(units: &mut Vec<UnitDefinition>);

        // Conversion utilities.
        type G4String;
        fn as_str(value: &G4String) -> &str;

        type G4ThreeVector;
        fn to_vec(value: &G4ThreeVector) -> [f64; 3];
        fn x(self: &G4ThreeVector) -> f64;
        fn y(self: &G4ThreeVector) -> f64;
        fn z(self: &G4ThreeVector) -> f64;
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
        fn envelope_shape(self: &Volume) -> &EnvelopeShape;
        fn is_rotated(self: &Volume) -> bool;
        fn is_translated(self: &Volume) -> bool;
        fn material(self: &Volume) -> &String;
        fn name(self: &Volume) -> &String;
        fn overlaps(self: &Volume) -> &[[String; 2]];
        fn position(self: &Volume) -> [f64; 3];
        fn rotation(self: &Volume) -> &[[f64; 3]];
        fn sensitive(self: &Volume) -> bool;
        fn shape(self: &Volume) -> ShapeType;
        fn sphere_shape(self: &Volume) -> &SphereShape;
        fn tessellated_shape(self: &Volume) -> &TessellatedShape;
        fn volumes(self: &Volume) -> &[Volume];

        type SortedTessels;
        fn sort_tessels(shape: &TessellatedShape) -> Box<SortedTessels>;

        fn area(self: &SortedTessels) -> f64;
        fn distance_to_in(
            self: &SortedTessels,
            point: &G4ThreeVector,
            direction: &G4ThreeVector
        ) -> f64;
        fn distance_to_out(
            self: &SortedTessels,
            point: &G4ThreeVector,
            direction: &G4ThreeVector,
            index: &mut i64,
        ) -> f64;
        fn envelope(self: &SortedTessels) -> [[f64; 3]; 2];
        fn inside(self: &SortedTessels, point: &G4ThreeVector, delta: f64) -> EInside;
        fn normal(self: &SortedTessels, index: usize) -> [f64; 3];
        fn surface_normal(self: &SortedTessels, point: &G4ThreeVector) -> [f64; 3];
        fn surface_point(self: &SortedTessels, index: f64, u: f64, v: f64) -> [f64; 3];

        // Materials interface.
        fn get_hash(self: &Mixture) -> u64;
        fn get_hash(self: &Molecule) -> u64;

        // Simulation interface.
        type RunAgent<'a>;

        fn events(self: &RunAgent) -> usize;
        unsafe fn geometry<'b>(self: &'b RunAgent) -> &'b GeometryBorrow;
        fn is_sampler(self: &RunAgent) -> bool;
        fn is_tracker(self: &RunAgent) -> bool;
        fn next_open01(self: &mut RunAgent) -> f64;
        unsafe fn next_primary<'b>(self: &'b mut RunAgent) -> &'b Primary;
        unsafe fn physics<'b>(self: &'b RunAgent) -> &'b Physics;
        fn prng_name(self: &RunAgent) -> &'static str;
        unsafe fn push_deposit(
            self: &mut RunAgent,
            volume: *const G4VPhysicalVolume,
            step_deposit: f64,
            non_ionising: f64,
            start: &G4ThreeVector,
            end: &G4ThreeVector,
        );
        fn push_track(self: &mut RunAgent, mut track: Track);
        fn push_vertex(self: &mut RunAgent, mut vertex: Vertex);
    }
}
