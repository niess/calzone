// User interface.
#include "geometry.h"
// Geant4 interface.
#include "G4RunManager.hh"


G4VPhysicalVolume * GeometryImpl::Construct() {
    auto && geometry = RUN_AGENT->geometry();
    return geometry.world();
}

void GeometryImpl::Reset() {
    auto manager = G4RunManager::GetRunManager();
    if (manager != nullptr) {
        manager->ReinitializeGeometry(false, true);
    }
}

void GeometryImpl::Update() {
    auto id = RUN_AGENT->geometry().id();
    if (id != this->geometry_id) {
        this->Reset();
    }
    this->geometry_id = id;
}

GeometryImpl * GeometryImpl::Get() {
    static GeometryImpl * instance = new GeometryImpl();
    return instance;
}
