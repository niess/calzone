// User interface.
#include "geometry.h"
// Geant4 interface.
#include "G4PVPlacement.hh"
#include "G4RunManager.hh"


G4VPhysicalVolume * DRY_WORLD = nullptr;
G4Material * DRY_WORLD_MATERIAL = nullptr;

G4VPhysicalVolume * GeometryImpl::Construct() {
    if (RUN_AGENT != nullptr) {
        auto && geometry = RUN_AGENT->geometry();
        return geometry.world();
    } else {
        // Dummy world for dry runs.
        static G4LogicalVolume * logical = nullptr;
        if (DRY_WORLD == nullptr) {
            G4Box * solid = new G4Box("World", 1.0, 1.0, 1.0);
            logical = new G4LogicalVolume(
                solid, DRY_WORLD_MATERIAL, "World"
            );
            DRY_WORLD = new G4PVPlacement(
                nullptr,
                G4ThreeVector(),
                logical,
                "World",
                nullptr,
                false,
                0
            );
        } else {
            logical->SetMaterial(DRY_WORLD_MATERIAL);
        }
        return DRY_WORLD;
    }
}

void GeometryImpl::Reset() {
    auto manager = G4RunManager::GetRunManager();
    if (manager != nullptr) {
        manager->ReinitializeGeometry(false, true);
    }
}

void GeometryImpl::Update() {
    if (RUN_AGENT != nullptr) {
        auto id = RUN_AGENT->geometry().id();
        if (id != this->geometry_id) {
            this->Reset();
        }
        this->geometry_id = id;
    }
}

GeometryImpl * GeometryImpl::Get() {
    static GeometryImpl * instance = new GeometryImpl();
    return instance;
}
