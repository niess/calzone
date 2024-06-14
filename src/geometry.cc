#include "calzone.h"
// standard library.
#include <list>
// fmt library.
#include <fmt/core.h>
// Geant4 interface.
#include "G4Box.hh"
#include "G4GDMLParser.hh"
#include "G4NistManager.hh"
#include "G4Orb.hh"
#include "G4PVPlacement.hh"
#include "G4SubtractionSolid.hh"
#include "G4TessellatedSolid.hh"
#include "G4TriangularFacet.hh"
#include "G4Tubs.hh"
#include "G4VisExtent.hh"
#include "G4VoxelLimits.hh"
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
    GeometryData(rust::Box<Volume> volume);
    ~GeometryData();

    GeometryData * clone();
    void drop();

    static GeometryData * get(const G4VPhysicalVolume *);

    G4VPhysicalVolume * world = nullptr;
    std::map<std::string, const G4VPhysicalVolume *> elements;

private:
    size_t rc = 0;
    std::list <G4VSolid *> orphans;
    static std::map<const G4VPhysicalVolume *, GeometryData *> INSTANCES;
};

std::map<const G4VPhysicalVolume *, GeometryData *> GeometryData::INSTANCES;

static G4TessellatedSolid * build_tessellation(const Volume & volume) {
    auto name = std::string(volume.name());
    auto solid = new G4TessellatedSolid(name);
    if (solid == nullptr) {
        set_error(ErrorType::MemoryError, "");
        return nullptr;
    }

    auto shape = volume.tessellated_shape();
    const size_t n = shape.facets.size() / 9;
    float * data = shape.facets.data();
    const float unit = (float)CLHEP::cm;
    for (size_t i = 0; i < n; i++, data += 9) {
        float * v0 = data;
        float * v1 = v0 + 3;
        float * v2 = v1 + 3;

        auto facet = new G4TriangularFacet(
            G4ThreeVector(v0[0] * unit, v0[1] * unit, v0[2] * unit),
            G4ThreeVector(v1[0] * unit, v1[1] * unit, v1[2] * unit),
            G4ThreeVector(v2[0] * unit, v2[1] * unit, v2[2] * unit),
            ABSOLUTE
        );
        if (!solid->AddFacet((G4VFacet *)facet)) {
            delete solid;
            auto message = fmt::format(
                "bad vertices for tessellation '{}'",
                name
            );
            set_error(ErrorType::ValueError, message.c_str());
            return nullptr;
        }
    }
    solid->SetSolidClosed(true);

    return solid;
}

static bool build_solids(
    const Volume & volume,
    const std::string & path,
    std::map<std::string, G4VSolid *> & solids,
    std::list<G4VSolid *> & orphans
) {
    auto name = std::string(volume.name());
    std::string pathname;
    if (path.empty()) {
        pathname = name;
    } else {
        pathname = fmt::format("{}.{}", path, name);
    }

    G4VSolid * solid = nullptr;
    switch (volume.shape()) {
        case ShapeType::Box: {
                auto shape = volume.box_shape();
                solid = new G4Box(
                    std::string(name),
                    0.5 * shape.size[0] * CLHEP::cm,
                    0.5 * shape.size[1] * CLHEP::cm,
                    0.5 * shape.size[2] * CLHEP::cm
                );
            }
            break;
        case ShapeType::Cylinder: {
                auto shape = volume.cylinder_shape();
                double rmin = (shape.thickness > 0.0) ?
                    shape.radius - shape.thickness : 0.0;
                solid = new G4Tubs(
                    std::string(name),
                    rmin * CLHEP::cm,
                    shape.radius * CLHEP::cm,
                    0.5 * shape.length * CLHEP::cm,
                    0.0,
                    CLHEP::twopi
                );
            }
            break;
        case ShapeType::Sphere: {
                auto shape = volume.sphere_shape();
                solid = new G4Orb(
                    std::string(name),
                    shape.radius * CLHEP::cm
                );
            }
            break;
        case ShapeType::Tessellation:
            solid = build_tessellation(volume);
            break;
    }
    if (solid == nullptr) {
        if (!any_error()) {
            auto msg = fmt::format(
                "bad '{}' volume (could not create solid)",
                pathname
            );
            set_error(ErrorType::ValueError, msg.c_str());
        }
        return false;
    }
    solids[pathname] = solid;

    // Build sub-solids.
    for (auto && v: volume.volumes()) {
        if (!build_solids(v, pathname, solids, orphans)) return false;
    }

    // Patch overlaps.
    for (auto overlap: volume.overlaps()) {
        const std::string path0 = fmt::format("{}.{}",
            pathname, std::string(overlap[0]));
        const std::string path1 = fmt::format("{}.{}",
            pathname, std::string(overlap[1]));
        auto solid0 = solids[path0];
        auto boolean = new G4SubtractionSolid(
            std::string(overlap[0]), solid0, solids[path1]
        );
        orphans.push_back(solid0);
        solids[path0] = boolean;
    }

    return true;
}

static void drop_them_all(const G4VPhysicalVolume * volume);

static void drop_them_all(G4LogicalVolume * logical) {
    // Delete any sub-volume(s).
    while (logical->GetNoDaughters()) {
        auto daughter = logical->GetDaughter(0);
        logical->RemoveDaughter(daughter);
        drop_them_all(daughter);
    }
    // Delete this volume.
    delete logical->GetSolid();
    delete logical;
}

static void drop_them_all(const G4VPhysicalVolume * volume) {
    // Delete any sub-volume(s).
    auto && logical = volume->GetLogicalVolume();
    drop_them_all(logical);
    delete volume;
}

static G4LogicalVolume * build_volumes(
    const Volume & volume,
    const std::string & path,
    std::map<std::string, G4VSolid *> & solids,
    std::map<std::string, const G4VPhysicalVolume *> & elements
) {
    auto name = std::string(volume.name());
    std::string pathname;
    if (path.empty()) {
        pathname = name;
    } else {
        pathname = fmt::format("{}.{}", path, name);
    }

    // Get material.
    G4Material * material = get_material(volume.material());
    if (material == nullptr) {
        auto msg = fmt::format(
            "bad '{}' volume (undefined '{}' material)",
            pathname,
            std::string(volume.material())
        );
        set_error(ErrorType::ValueError, msg.c_str());
        return nullptr;
    }

    // Get solid.
    auto i = solids.find(pathname);
    G4VSolid * solid = std::move(i->second);
    solids.erase(i);

    // Build logical volume.
    auto logical = new G4LogicalVolume(solid, material, name);
    if (logical == nullptr) {
        delete solid;
        auto msg = fmt::format(
            "bad '{}' volume (could not create logical volume)",
            pathname
        );
        set_error(ErrorType::ValueError, msg.c_str());
        return nullptr;
    }

    // XXX Set any sensitive detector.

    // Build sub-volumes.
    for (auto && v: volume.volumes()) {
        auto l = build_volumes(v, pathname, solids, elements);
        if (l == nullptr) {
            drop_them_all(logical);
            return nullptr;
        }

        auto && p = v.position();
        G4RotationMatrix * rotation = nullptr;
        if (v.is_rotated()) {
            auto && m = v.rotation();
            auto rowX = G4ThreeVector(m[0][0], m[0][1], m[0][2]);
            auto rowY = G4ThreeVector(m[1][0], m[1][1], m[1][2]);
            auto rowZ = G4ThreeVector(m[2][0], m[2][1], m[2][2]);
            rotation->setRows(rowX, rowY, rowZ);
        }
        auto position = G4ThreeVector(
            p[0] * CLHEP::cm,
            p[1] * CLHEP::cm,
            p[2] * CLHEP::cm
        );
        auto v_name = std::string(v.name());
        auto v_path = fmt::format("{}.{}", pathname, v_name);
        elements[v_path] = new G4PVPlacement(
            rotation,
            position,
            l,
            v_name,
            logical,
            false,
            0
        );
    }

    return logical;
}

GeometryData::GeometryData(rust::Box<Volume> volume) {
    clear_error();
    this->world = nullptr;

    // Build solids.
    std::map<std::string, G4VSolid *> solids;
    const std::string path = "";
    if (!build_solids(*volume, path, solids, this->orphans)) {
        for (auto item: solids) {
            delete item.second;
        }
        return;
    }

    // Build volumes.
    auto logical = build_volumes(*volume, path, solids, this->elements);
    if (logical == nullptr) {
        for (auto item: solids) {
            delete item.second;
        }
        for (auto solid: this->orphans) {
            delete solid;
        }
        this->orphans.clear();
        return;
    } else {
        // At this stage, solids should have been all consumed.
        assert(solids.empty());
    }

    // Register the world volume.
    auto world_name = std::string(volume->name());
    this->world = new G4PVPlacement(
        nullptr,
        G4ThreeVector(0.0, 0.0, 0.0),
        logical,
        world_name,
        nullptr,
        false,
        0
    );
    this->elements[world_name] = this->world;
    this->INSTANCES[this->world] = this;
}

GeometryData::~GeometryData() {
    if (this->world != nullptr) {
        this->INSTANCES.erase(this->world);
        drop_them_all(this->world);
        for (auto solid: this->orphans) {
            delete solid;
        }
        this->orphans.clear();
        this->elements.clear();
    }
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

std::shared_ptr<GeometryBorrow> create_geometry(rust::Box<Volume> volume) {
    auto data = new GeometryData(std::move(volume));
    if (any_error()) {
        delete data;
        return nullptr;
    } else {
        return std::make_shared<GeometryBorrow>(data);
    }
}


// ============================================================================
//
// Geant4 interface.
//
// ============================================================================

static void check_overlaps(G4VPhysicalVolume * volume, int resolution) {
    volume->CheckOverlaps(resolution, 0.0, false);
    if (any_error()) return;

    auto && logical = volume->GetLogicalVolume();
    int n = logical->GetNoDaughters();
    for (int i = 0; i < n; i++) {
        auto daughter = logical->GetDaughter(i);
        check_overlaps(daughter, resolution);
        if (any_error()) return;
    }
}

std::shared_ptr<Error> GeometryBorrow::check(int resolution) const {
    clear_error();
    check_overlaps(this->data->world, resolution);
    return get_error();
}

static const G4VPhysicalVolume * get_volume(
    std::string & path,
    std::map<std::string, const G4VPhysicalVolume *> & elements
) {
    // XXX This (and below 'get_transform') is fragile for imported GDML.

    auto volume = elements[path];
    if (volume == nullptr) {
        auto msg = fmt::format("unknown volume '{}'", path);
        set_error(ErrorType::ValueError, msg.c_str());
    }
    return volume;
}

std::pair<G4AffineTransform, const G4VPhysicalVolume *> get_transform(
    std::string & volume,
    std::string & frame,
    std::map<std::string, const G4VPhysicalVolume *> & elements
) {
    auto transform = G4AffineTransform();
    if (volume == frame) {
        return std::make_pair(transform, nullptr);
    }

    auto current = std::string(frame);
    if (volume.rfind(current, 0) != 0) {
        auto msg = fmt::format(
            "'{}' does not contain '{}'",
            current,
            volume
        );
        set_error(ErrorType::ValueError, msg.c_str());
        return std::make_pair(transform, nullptr);
    }

    std::istringstream remainder(volume.substr(current.size() + 1));
    const G4VPhysicalVolume * ref;
    for (;;) {
        ref = get_volume(current, elements);
        if (ref == nullptr) {
            return std::make_pair(transform, nullptr);
        }

        transform *= G4AffineTransform(
            ref->GetRotation(),
            ref->GetTranslation()
        );

        std::string stem;
        std::getline(remainder, stem, '.');
        if (stem.empty()) break;
        current = fmt::format("{}.{}", current, stem);
    }

    return std::make_pair(transform, ref);
}

std::array<double, 6> GeometryBorrow::compute_box(
    rust::Str volume_,
    rust::Str frame_
) const {
    std::array<double, 6> box = { 0.0, 0.0, 0.0, 0.0, 0.0, 0.0 };
    auto volume = std::string(volume_);
    std::string frame;
    if (frame_.empty()) {
        frame = this->data->world->GetName();
    } else {
        frame = std::string(frame_);
    }

    auto result = get_transform(volume, frame, this->data->elements);
    if (any_error()) {
        return box;
    }

    G4AffineTransform transform = std::move(result.first);
    const G4VPhysicalVolume * physical = std::move(result.second);
    if (physical == nullptr) {
        physical = get_volume(volume, this->data->elements);
        if (physical == nullptr) {
            return box;
        }
    }

    auto solid = physical->GetLogicalVolume()->GetSolid();
    if (transform.IsTranslated() || transform.IsRotated()) {
        auto limits = G4VoxelLimits();
        solid->CalculateExtent(kXAxis, limits, transform, box[0], box[1]);
        solid->CalculateExtent(kYAxis, limits, transform, box[2], box[3]);
        solid->CalculateExtent(kZAxis, limits, transform, box[4], box[5]);
    } else {
        auto extent = solid->GetExtent();
        box[0] = extent.GetXmin();
        box[1] = extent.GetXmax();
        box[2] = extent.GetYmin();
        box[3] = extent.GetYmax();
        box[4] = extent.GetZmin();
        box[5] = extent.GetZmax();
    }

    for (auto && value: box) {
        value /= CLHEP::cm;
    }

    return box;
}

std::array<double, 3> GeometryBorrow::compute_origin(
    rust::Str volume_,
    rust::Str frame_
) const {
    std::array<double, 3> origin = { 0.0, 0.0, 0.0 };
    auto volume = std::string(volume_);
    std::string frame;
    if (frame_.empty()) {
        frame = this->data->world->GetName();
    } else {
        frame = std::string(frame_);
    }

    auto result = get_transform(volume, frame, this->data->elements);
    if (any_error()) {
        return origin;
    }

    G4AffineTransform transform = std::move(result.first);
    auto p = transform.TransformPoint(G4ThreeVector(0.0, 0.0, 0.0));
    for (auto i = 0; i < 3; i++) {
        origin[i] = p[i] / CLHEP::cm;
    }

    return origin;
}

VolumeInfo GeometryBorrow::describe_volume(rust::Str name_) const {
    VolumeInfo info;
    auto name = std::string(name_);
    auto volume = get_volume(name, this->data->elements);
    if (volume != nullptr) {
        auto logical = volume->GetLogicalVolume();
        info.material = rust::String(logical->GetMaterial()->GetName());
        info.solid = rust::String(logical->GetSolid()->GetName());
    }
    return info;
}

std::shared_ptr<Error> GeometryBorrow::dump(rust::Str path) const {
    clear_error();
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
