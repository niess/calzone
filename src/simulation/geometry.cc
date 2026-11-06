// User interface.
#include "geometry.h"
// Geant4 interface.
#include "G4PVPlacement.hh"
#include "G4RunManager.hh"


static const char * DRY_WORLD_NAME = "__DryWorld__";

[[noreturn]] static void unreachable() {
    std::cerr << "code has reached an unexpected state" << std::endl;
    std::exit(EXIT_FAILURE);
}

G4VPhysicalVolume * GeometryImpl::Construct() {
    if (this->state == GeometryState::Dry) {
        return this->dry_world;
    } else if (this->state == GeometryState::Run) {
        auto && geometry = RUN_AGENT->geometry();
        return geometry.world();
    } else {
        unreachable();
    }
}

void GeometryImpl::Reset(G4VPhysicalVolume * world) const {
    auto manager = G4RunManager::GetRunManager();
    if (manager != nullptr) {
        if (world != nullptr) {
            manager->DefineWorldVolume(world);
        }
        manager->ReinitializeGeometry(false);
    }
}

void GeometryImpl::SetDryMaterial(G4Material * material) {
    this->dry_material = material;
    this->dry_logical->SetMaterial(material);
}

void GeometryImpl::Update() {
    if (this->state == GeometryState::None) {
        if (RUN_AGENT != nullptr) {
            this->state = GeometryState::Run;
        } else {
            this->SetDryMaterial(PhysicsCalculator::DRY_WORLD_MATERIAL);
            this->state = GeometryState::Dry;
        }
    } else {
        if (RUN_AGENT != nullptr) {
            auto id = RUN_AGENT->geometry().id();
            bool update = false;
            if (this->state == GeometryState::Run) {
                if (id != this->geometry_id) {
                    update = true;
                }
            } else if (this->state == GeometryState::Dry) {
                this->state = GeometryState::Run;
                this->dry_material = nullptr;
                update = true;
            } else {
                unreachable();
            }
            if (update) {
                auto && world = RUN_AGENT->geometry().world();
                this->Reset(world);
                this->geometry_id = id;
            }
        } else if (PhysicsCalculator::DRY_WORLD_MATERIAL != nullptr) {
            if (this->state == GeometryState::Run) {
                this->SetDryMaterial(PhysicsCalculator::DRY_WORLD_MATERIAL);
                this->Reset(this->dry_world);
            } else if (this->state == GeometryState::Dry) {
                if (PhysicsCalculator::DRY_WORLD_MATERIAL !=
                    this->dry_material) {
                    this->SetDryMaterial(PhysicsCalculator::DRY_WORLD_MATERIAL);
                    auto manager = G4RunManager::GetRunManager();
                    manager->GeometryHasBeenModified();
                }
            } else {
                unreachable();
            }
            this->state = GeometryState::Dry;
        }
    }
}

GeometryImpl * GeometryImpl::Get() {
    static GeometryImpl * instance = nullptr;
    if (instance == nullptr) {
        instance = new GeometryImpl();

        // Construct a dummy world for dry runs.
        G4Box * solid = new G4Box(DRY_WORLD_NAME, 1.0, 1.0, 1.0);
        instance->dry_logical = new G4LogicalVolume(
            solid, nullptr, DRY_WORLD_NAME
        );
        instance->dry_world = new G4PVPlacement(
            nullptr,
            G4ThreeVector(),
            instance->dry_logical,
            DRY_WORLD_NAME,
            nullptr,
            false,
            0
        );
    }
    return instance;
}
