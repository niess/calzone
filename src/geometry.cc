#include "calzone.h"
// Geant4 interface.
#include "G4Box.hh"
#include "G4GDMLParser.hh"
#include "G4LogicalVolume.hh" // XXX needed?
#include "G4NistManager.hh"
#include "G4PVPlacement.hh"
// Goupil interface.
#include "G4Goupil.hh"


// ============================================================================
//
// Managed geometry data.
//
// This is basically a reference counted G4VPhysicalVolume with bookkeeping of
// allocated data.
//
// ============================================================================

struct GeometryData {
    GeometryData();
    ~GeometryData();

    GeometryData * clone();
    void drop();

    static GeometryData * get(const G4VPhysicalVolume *);

    G4VPhysicalVolume * world = nullptr;

private:
    size_t rc = 0;
    static std::map<const G4VPhysicalVolume *, GeometryData *> INSTANCES;
};

std::map<const G4VPhysicalVolume *, GeometryData *> GeometryData::INSTANCES;

GeometryData::GeometryData() {
    auto manager = G4NistManager::Instance();
    std::string name = "World";
    auto solid = new G4Box(
            name,
            0.5,
            0.5,
            0.5
    );
    auto material = manager->FindOrBuildMaterial("G4_AIR");
    auto logical = new G4LogicalVolume(solid, material, name);
    this->world = new G4PVPlacement(
        nullptr,
        G4ThreeVector(0.0, 0.0, 0.0),
        logical,
        name,
        nullptr,
        false,
        0
    );

    this->INSTANCES[this->world] = this;
}

static void drop_them_all(const G4VPhysicalVolume * volume) {
    // Delete any sub-volume(s).
    auto && logical = volume->GetLogicalVolume();
    while (logical->GetNoDaughters()) {
        auto daughter = logical->GetDaughter(0);
        logical->RemoveDaughter(daughter);
        drop_them_all(daughter);
    }
    // Delete this volume.
    delete logical;
    delete volume;
}

GeometryData::~GeometryData() {
    this->INSTANCES.erase(this->world);
    drop_them_all(this->world); // XXX delete orphans as well.
}

GeometryData * GeometryData::clone() {
    this->rc++;
    return this;
}

void GeometryData::drop() {
    if (--this->rc == 0) delete this;
}

GeometryData * GeometryData::get(const G4VPhysicalVolume * volume) {
    return GeometryData::INSTANCES[volume];
}


// ============================================================================
//
// Borrow interface.
//
// This is a wrapper for Rust. Directly forwarding the geometry data would
// result in Rust deleting them when dropping the shared pointer. Dropping the
// wrapper instead results in data being deleted iff there are no pending
// references.
//
// ============================================================================

GeometryBorrow::GeometryBorrow(GeometryData * d) {
    this->data = d->clone();
}

GeometryBorrow::~GeometryBorrow() {
    this->data->drop();
}

std::shared_ptr<GeometryBorrow> create_geometry() {
    auto data = new GeometryData;
    return std::make_shared<GeometryBorrow>(data);
}


// ============================================================================
//
// GDML interface.
//
// ============================================================================

std::shared_ptr<Error> GeometryBorrow::dump(rust::Str path) const {
    G4GDMLParser parser;
    auto buffer = std::cout.rdbuf();
    std::cout.rdbuf(nullptr); // Disable cout temporarly.
    parser.Write(std::string(path), this->data->world);
    std::cout.rdbuf(buffer);
    return get_error();
}


// ============================================================================
//
// Goupil interface.
//
// ============================================================================

static GeometryData * GOUPIL_GEOMETRY = nullptr;

void GeometryBorrow::set_goupil() const {
    GOUPIL_GEOMETRY = this->data;
}

const G4VPhysicalVolume * G4Goupil::NewGeometry() {
    auto geometry = GOUPIL_GEOMETRY->clone();
    return geometry->world;
}

void G4Goupil::DropGeometry(const G4VPhysicalVolume * volume) {
    auto geometry = GeometryData::get(volume);
    geometry->drop();
}
