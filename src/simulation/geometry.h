#pragma once
// Geant4 interface.
#include "G4VUserDetectorConstruction.hh"
// User interface.
#include "calzone.h"


enum class GeometryState {
    Dry,
    None,
    Run,
};

struct GeometryImpl: public G4VUserDetectorConstruction {
    GeometryImpl(const GeometryImpl &) = delete;

    // Geant4 interface.
    G4VPhysicalVolume * Construct();

    // User interface.
    void Reset();
    void Update();

    GeometryState State() const {
        return this->state;
    }

    static GeometryImpl * Get();

private:
    GeometryImpl() = default;

    // User interface.
    std::uint64_t geometry_id = 0;
    GeometryState state = GeometryState::None;
    G4LogicalVolume * dry_logical = nullptr;
    G4Material * dry_material = nullptr;
    G4VPhysicalVolume * dry_world = nullptr;
};
