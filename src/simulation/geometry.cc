// User interface.
#include "geometry.h"
// Geant4 interface.
#include "G4PVPlacement.hh"
#include "G4RunManager.hh"


G4VPhysicalVolume * GeometryImpl::Construct() {
    if (RUN_AGENT != nullptr) {
        std::cout << "XXX Building run geometry " << this->geometry_id << std::endl;
        auto && geometry = RUN_AGENT->geometry();
        return geometry.world();
    } else {
        if (this->dry_world == nullptr) {
            std::cout << "XXX Building dry geometry for " << this->dry_material->GetName() << std::endl;

            // Construct Dummy world for dry runs.
            G4Box * solid = new G4Box("World", 1.0, 1.0, 1.0);
            this->dry_logical = new G4LogicalVolume(
                solid, this->dry_material, "World"
            );
            this->dry_world = new G4PVPlacement(
                nullptr,
                G4ThreeVector(),
                this->dry_logical,
                "World",
                nullptr,
                false,
                0
            );
        } else if (this->dry_logical->GetMaterial() != this->dry_material) {
            // Update material.
            std::cout << "XXX Updating dry geometry for " << this->dry_material->GetName() << std::endl;
            this->dry_logical->SetMaterial(this->dry_material);
        }
        return this->dry_world;
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
        if (this->state == GeometryState::Run) {
            if (id != this->geometry_id) {
                this->Reset();
            }
        } else {
            this->state = GeometryState::Run;
            this->dry_material = nullptr;
            this->Reset();
        }
        this->geometry_id = id;
    } else if (PhysicsCalculator::DRY_WORLD_MATERIAL != nullptr) {
        if (PhysicsCalculator::DRY_WORLD_MATERIAL != this->dry_material) {
            this->dry_material = PhysicsCalculator::DRY_WORLD_MATERIAL;
            this->Reset();
            std::cout << "XXX Setting dry material to " << this->dry_material->GetName() << std::endl;
        }
        this->state = GeometryState::Dry;
    }
}

GeometryImpl * GeometryImpl::Get() {
    static GeometryImpl * instance = new GeometryImpl();
    return instance;
}
