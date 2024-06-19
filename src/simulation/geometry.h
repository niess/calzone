#pragma once
// Geant4 interface.
#include "G4VUserDetectorConstruction.hh"
// User interface.
#include "calzone.h"


struct GeometryImpl: public G4VUserDetectorConstruction {
    GeometryImpl(const GeometryImpl &) = delete;

    // Geant4 interface.
    G4VPhysicalVolume * Construct();

    // User interface.
    void Reset();
    void Update();

    static GeometryImpl * Get();

private:
    GeometryImpl() = default;

    // User interface.
    size_t geometry_id = 0;
};
