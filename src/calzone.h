#pragma once
// Standard library.
#include <memory>
// Geant4 interface.
#include "G4Material.hh"
#include "G4VPhysicalVolume.hh"
// Calzone interface.
struct GeometryBorrow;
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

    // Geant4 interface.
    std::shared_ptr<Error> check(int resolution) const;
    std::array<double, 6> compute_box(rust::Str, rust::Str) const;
    std::array<double, 3> compute_origin(rust::Str, rust::Str) const;
    VolumeInfo describe_volume(rust::Str) const;
    std::shared_ptr<Error> dump(rust::Str) const;
    size_t id() const;
    G4VPhysicalVolume * world() const;

    // Volume roles interface.
    std::shared_ptr<Error> clear_roles(rust::Str) const;
    Roles get_roles(rust::Str) const;
    std::shared_ptr<Error> set_roles(rust::Str, Roles) const;

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
