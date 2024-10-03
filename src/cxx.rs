use crate::geometry::volume::Volume;
use crate::geometry::tessellation::{SortedTessels, sort_tessels};
use crate::simulation::{RandomContext, RunAgent};
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

    #[derive(Deserialize, Serialize)]
    struct BoxShape {
        size: [f64; 3],
    }

    #[derive(Deserialize, Serialize)]
    struct CylinderShape {
        radius: f64,
        length: f64,
        thickness: f64,
        section: [f64; 2],
    }

    #[repr(u32)]
    enum EInside {
      kOutside,
      kSurface,
      kInside,
    }

    #[derive(Deserialize, Serialize)]
    struct EnvelopeShape {
        shape: ShapeType,
        padding: [f64; 6],
    }

    struct DaughterInfo {
        path: String,
        solid: String,
    }

    #[derive(Deserialize, Serialize)]
    #[repr(i32)]
    #[serde(transparent)]
    enum ShapeType {
        Box,
        Cylinder,
        Envelope,
        Sphere,
        Tessellation,
    }

    #[derive(Deserialize, Serialize)]
    struct SphereShape {
        radius: f64,
        thickness: f64,
        azimuth_section: [f64; 2],
        zenith_section: [f64; 2],
    }

    #[derive(Deserialize, Serialize)]
    struct TessellatedShape {
        facets: Vec<f32>,
    }

    #[repr(i32)]
    enum TSTAlgorithm {
        Bvh,
        Geant4,
    }

    struct VolumeInfo { // From Geant4.
        path: String,
        material: String,
        solid: String,
        mother: String,
        daughters: Vec<DaughterInfo>,
    }

    #[derive(Serialize)]
    struct BoxInfo {
        size: [f64; 3],
        displacement: [f64; 3],
    }

    #[derive(Serialize)]
    struct OrbInfo {
        radius: f64,
        displacement: [f64; 3],
    }

    #[derive(Serialize)]
    struct SphereInfo {
        inner_radius: f64,
        outer_radius: f64,
        start_phi_angle: f64,
        delta_phi_angle: f64,
        start_theta_angle: f64,
        delta_theta_angle: f64,
    }

    #[derive(Serialize)]
    struct TransformInfo {
        translation: [f64; 3],
        rotation: [[f64; 3]; 3],
    }

    #[derive(Serialize)]
    struct TubsInfo {
        inner_radius: f64,
        outer_radius: f64,
        length: f64,
        start_phi_angle: f64,
        delta_phi_angle: f64,
        displacement: [f64; 3],
    }


    // ===========================================================================================
    //
    // Materials interface.
    //
    // ===========================================================================================

    #[derive(Deserialize, Serialize)]
    struct Element {
        name: String,
        symbol: String,
        Z: f64,
        A: f64,
    }

    #[derive(Deserialize, Serialize)]
    #[repr(u32)]
    #[serde(transparent)]
    enum G4State {
      kStateUndefined = 0,
      kStateSolid,
      kStateLiquid,
      kStateGas
    }

    #[derive(Deserialize, Serialize)]
    struct MaterialProperties {
        name: String,
        density: f64,
        state: G4State,
    }

    #[derive(Hash, Deserialize, Serialize)]
    struct Mixture {
        properties: MaterialProperties,
        components: Vec<MixtureComponent>,
    }

    #[derive(PartialEq, PartialOrd, Deserialize, Serialize)]
    struct MixtureComponent {
        name: String,
        weight: f64,
    }

    #[derive(Hash, Deserialize, Serialize)]
    struct Molecule {
        properties: MaterialProperties,
        components: Vec<MoleculeComponent>,
    }

    #[derive(Hash, PartialEq, PartialOrd, Deserialize, Serialize)]
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
    // Sampler interface.
    //
    // ===========================================================================================

    #[derive(Deserialize, Serialize)]
    #[repr(u32)]
    #[serde(transparent)]
    enum Action {
        None = 0,
        Catch,
        Kill,
        Record,
    }

    #[derive(Clone, Copy, Deserialize, Serialize)]
    struct Roles {
        ingoing: Action,
        outgoing: Action,
        deposits: Action,
    }

    #[derive(Clone, Copy)]
    struct SampledParticle {
        event: usize,
        state: Particle,
    }

    // ===========================================================================================
    //
    // Source interface.
    //
    // ===========================================================================================

    #[derive(Clone, Copy)]
    struct Particle {
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
        volume: [u8; 16],
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
        type G4AffineTransform;

        type GeometryBorrow;
        fn create_geometry(
            volume: &Box<Volume>,
            algorithm: &TSTAlgorithm,
        ) -> SharedPtr<GeometryBorrow>;

        fn borrow_volume(self: &GeometryBorrow, name: &str) -> SharedPtr<VolumeBorrow>;
        fn check(self: &GeometryBorrow, resolution: i32) -> SharedPtr<Error>;
        fn dump(self: &GeometryBorrow, path: &str) -> SharedPtr<Error>;
        fn find_volume(self: &GeometryBorrow, stem: &str) -> SharedPtr<VolumeBorrow>;
        fn set_goupil(self: &GeometryBorrow);

        type VolumeBorrow;
        fn compute_box(self: &VolumeBorrow, frame: &str) -> [f64; 6];
        fn compute_transform(self: &VolumeBorrow, frame: &str) -> UniquePtr<G4AffineTransform>;
        fn compute_origin(self: &VolumeBorrow, frame: &str) -> [f64; 3];
        fn compute_surface(self: &VolumeBorrow) -> f64;
        fn compute_volume(self: &VolumeBorrow, include_daughters: bool) -> f64;
        fn describe(self: &VolumeBorrow) -> VolumeInfo;
        fn describe_box(self: &VolumeBorrow) -> BoxInfo;
        fn describe_orb(self: &VolumeBorrow) -> OrbInfo;
        fn describe_sphere(self: &VolumeBorrow) -> SphereInfo;
        fn describe_tessellated_solid(self: &VolumeBorrow, vertices: &mut Vec<f32>);
        fn describe_tessellation(self: &VolumeBorrow) -> &Box<SortedTessels>;
        fn describe_transform(self: &VolumeBorrow) -> TransformInfo;
        fn describe_tubs(self: &VolumeBorrow) -> TubsInfo;
        fn dump(self: &VolumeBorrow, path: &str) -> SharedPtr<Error>;
        fn generate_onto(
            self: &VolumeBorrow,
            random: &mut RandomContext,
            transform: &G4AffineTransform,
            compute_normal: bool
        ) -> [f64; 6];
        fn inside(
            self: &VolumeBorrow,
            point: &[f64; 3],
            transform: &G4AffineTransform,
            include_daughters: bool
        ) -> EInside;

        fn clear_roles(self: &VolumeBorrow);
        fn get_roles(self: &VolumeBorrow) -> Roles;
        fn set_roles(self: &VolumeBorrow, roles: Roles);

        // Material interface.
        type G4State;
        fn add_element(element: &Element) -> SharedPtr<Error>;
        fn add_mixture(element: &Mixture) -> SharedPtr<Error>;
        fn add_molecule(element: &Molecule) -> SharedPtr<Error>;

        // Simulation interface.
        fn drop_simulation();
        fn run_simulation(
            agent: &mut RunAgent,
            random: &mut RandomContext,
            verbose: bool
        ) -> SharedPtr<Error>;

        type G4VPhysicalVolume;
        fn GetName(self: &G4VPhysicalVolume) -> &G4String;

        // Random interface.
        fn set_random_context(context: &mut RandomContext);
        fn release_random_context();

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
        fn roles(self: &Volume) -> Roles;
        fn rotation(self: &Volume) -> &[[f64; 3]];
        fn sensitive(self: &Volume) -> bool;
        fn shape(self: &Volume) -> ShapeType;
        fn sphere_shape(self: &Volume) -> &SphereShape;
        fn subtract(self: &Volume) -> &[String];
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
        fn surface_normal(self: &SortedTessels, point: &G4ThreeVector, delta: f64) -> [f64; 3];
        fn surface_point(self: &SortedTessels, index: f64, u: f64, v: f64) -> [f64; 3];

        // Materials interface.
        fn get_hash(self: &Mixture) -> u64;
        fn get_hash(self: &Molecule) -> u64;

        // Simulation interface.
        type RunAgent<'a>;

        fn events(self: &RunAgent) -> usize;
        unsafe fn geometry<'b>(self: &'b RunAgent) -> &'b GeometryBorrow;
        fn is_deposits(self: &RunAgent) -> bool;
        fn is_particles(self: &RunAgent) -> bool;
        fn is_secondaries(self: &RunAgent) -> bool;
        fn is_tracker(self: &RunAgent) -> bool;
        unsafe fn next_primary(self: &mut RunAgent) -> Particle;
        unsafe fn physics<'b>(self: &'b RunAgent) -> &'b Physics;
        unsafe fn push_deposit(
            self: &mut RunAgent,
            volume: *const G4VPhysicalVolume,
            step_deposit: f64,
            non_ionising: f64,
            start: &G4ThreeVector,
            end: &G4ThreeVector,
        );
        unsafe fn push_particle(
            self: &mut RunAgent,
            volume: *const G4VPhysicalVolume,
            mut particle: Particle,
        );
        fn push_track(self: &mut RunAgent, mut track: Track);
        fn push_vertex(self: &mut RunAgent, mut vertex: Vertex);

        // Random interface.
        type RandomContext<'a>;

        fn next_open01(self: &mut RandomContext) -> f64;
        fn prng_name(self: &RandomContext) -> &'static str;
    }
}
