#pragma once
// Standard library.
#include <memory>
// Geant4 interface.
#include "G4Material.hh"
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

    // GDML interface.
    std::shared_ptr<Error> dump(rust::Str) const;

    // Goupil interface.
    void set_goupil() const;

private:
    GeometryData * data;
};

std::shared_ptr<GeometryBorrow> create_geometry();

std::shared_ptr<Error> add_element(const Element &);
std::shared_ptr<Error> add_mixture(const Mixture &);
std::shared_ptr<Error> add_molecule(const Molecule &);
