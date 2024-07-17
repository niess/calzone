#pragma once
// Standard library.
#include <memory>
// Geant4 interface.
#include "G4AffineTransform.hh"
#include "G4Material.hh"
#include "G4VPhysicalVolume.hh"
// Calzone interface.
struct GeometryBorrow;
struct VolumeBorrow;
#include "calzone/src/cxx.rs.h"


// ============================================================================
//
// Error interface.
//
// ============================================================================

bool any_error();
void clear_error();
std::shared_ptr<Error> get_error();
void initialise_errors();
void set_error(ErrorType, const char *);


// ============================================================================
//
// Geometry interface.
//
// ============================================================================

struct GeometryData;

struct GeometryBorrow {
    GeometryBorrow(GeometryData *);
    ~GeometryBorrow();

    GeometryBorrow(const GeometryBorrow &) = delete; // Forbid copy.

    // User interface.
    std::shared_ptr<VolumeBorrow> borrow_volume(rust::Str) const;

    // Geant4 interface.
    std::shared_ptr<Error> check(int resolution) const;
    std::shared_ptr<Error> dump(rust::Str) const;
    size_t id() const;
    G4VPhysicalVolume * world() const;

    // Goupil interface.
    void set_goupil() const;

private:
    GeometryData * data;
};

std::shared_ptr<GeometryBorrow> create_geometry(
    const rust::Box<Volume> &,
    const TSTAlgorithm &
);


// ============================================================================
//
// Volume interface.
//
// ============================================================================

struct VolumeBorrow {
    VolumeBorrow(GeometryData *, const G4VPhysicalVolume *);
    ~VolumeBorrow();
    VolumeBorrow(const VolumeBorrow &) = delete; // Forbid copy.

    // User interface.
    std::array<double, 6> compute_box(rust::Str) const;
    std::unique_ptr<G4AffineTransform> compute_transform(rust::Str) const;
    std::array<double, 3> compute_origin(rust::Str) const;
    double compute_volume(bool) const;
    VolumeInfo describe() const;
    EInside inside(
        const std::array<double, 3> &,
        const G4AffineTransform &,
        bool
    ) const;

    // Roles interface.
    void clear_roles() const;
    Roles get_roles() const;
    void set_roles(Roles) const;

private:
    GeometryData * geometry;
    const G4VPhysicalVolume * volume;
};


// ============================================================================
//
// Materials interface.
//
// ============================================================================

std::shared_ptr<Error> add_element(const Element &);
std::shared_ptr<Error> add_mixture(const Mixture &);
std::shared_ptr<Error> add_molecule(const Molecule &);

class G4Material;
G4Material * get_material(const rust::String & name);


// ============================================================================
//
// Simulation interface.
//
// ============================================================================

extern RunAgent * RUN_AGENT;

void drop_simulation();
std::shared_ptr<Error> run_simulation(RunAgent & agent, bool verbose);


// ============================================================================
//
// Units interface.
//
// ============================================================================

void export_units(rust::Vec<UnitDefinition> & units);


// ============================================================================
//
// Conversion utilities.
//
// ============================================================================

rust::Str as_str(const G4String &);
std::array<double, 3> to_vec(const G4ThreeVector &);
