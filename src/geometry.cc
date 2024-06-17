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
    std::map<const G4VPhysicalVolume *, const G4VPhysicalVolume *> mothers;

private:
    size_t rc = 0;
    std::list <G4VSolid *> orphans;
    static std::map<const G4VPhysicalVolume *, GeometryData *> INSTANCES;
};

std::map<const G4VPhysicalVolume *, GeometryData *> GeometryData::INSTANCES;

static G4VSolid * build_envelope(
    const std::string & pathname,
    const Volume & volume,
    std::list<const G4VSolid *> & daughters,
    std::list<G4VSolid *> & orphans
) {
    auto get_transform = [&](const Volume & v) -> G4AffineTransform {
        auto && p = v.position();
        auto translation = G4ThreeVector(
            p[0] * CLHEP::cm,
            p[1] * CLHEP::cm,
            p[2] * CLHEP::cm
        );
        if (v.is_rotated()) {
            G4RotationMatrix rotation;
            auto && m = v.rotation();
            auto rowX = G4ThreeVector(m[0][0], m[0][1], m[0][2]);
            auto rowY = G4ThreeVector(m[1][0], m[1][1], m[1][2]);
            auto rowZ = G4ThreeVector(m[2][0], m[2][1], m[2][2]);
            rotation.setRows(rowX, rowY, rowZ);
            return G4AffineTransform(rotation, translation);
        } else {
            return G4AffineTransform(translation);
        }
    };

    // Compute limits along X, Y and Z axis.
    auto envelope = volume.envelope_shape();
    std::array<double, 3> min = { DBL_MAX, DBL_MAX, DBL_MAX };
    std::array<double, 3> max = { -DBL_MAX, -DBL_MAX, -DBL_MAX };
    for (auto && v: volume.volumes()) {
        std::array<double, 3> mi;
        std::array<double, 3> mx;
        const G4VSolid * s = daughters.front();
        daughters.pop_front();
        auto t = get_transform(v);
        if (t.IsTranslated() || t.IsRotated()) {
            auto l = G4VoxelLimits();
            s->CalculateExtent(kXAxis, l, t, mi[0], mx[0]);
            s->CalculateExtent(kYAxis, l, t, mi[1], mx[1]);
            s->CalculateExtent(kZAxis, l, t, mi[2], mx[2]);
        } else {
            auto extent = s->GetExtent();
            mi[0] = extent.GetXmin();
            mx[0] = extent.GetXmax();
            mi[1] = extent.GetYmin();
            mx[1] = extent.GetYmax();
            mi[2] = extent.GetZmin();
            mx[2] = extent.GetZmax();
        }
        for (size_t i = 0; i < mi.size(); i++) {
            if (mi[i] < min[i]) min[i] = mi[i];
            if (mx[i] > max[i]) max[i] = mx[i];
        }
    }
    auto safety = envelope.safety * CLHEP::cm;

    // Create bounding solid.
    G4VSolid * solid;
    switch (envelope.shape) {
        case ShapeType::Box:
            solid = new G4Box(
                pathname,
                0.5 * (max[0] - min[0]) + safety,
                0.5 * (max[1] - min[1]) + safety,
                0.5 * (max[2] - min[2]) + safety
            );
            break;
        case ShapeType::Cylinder: {
                const double dx = max[0] - min[0];
                const double dy = max[1] - min[1];
                const double radius = 0.5 * std::sqrt(dx * dx + dy * dy);
                solid = new G4Tubs(
                    pathname,
                    0.0,
                    radius + safety,
                    0.5 * (max[2] - min[2]) + safety,
                    0.0,
                    CLHEP::twopi
                );
            }
            break;
        case ShapeType::Sphere: {
                const double dx = max[0] - min[0];
                const double dy = max[1] - min[1];
                const double dz = max[2] - min[2];
                const double radius =
                    0.5 * std::sqrt(dx * dx + dy * dy + dz * dz);
                solid = new G4Orb(
                    pathname,
                    radius + safety
                );
            }
            break;
        default:
            return nullptr; // unreachable
    }

    // Translate solid, if not already centered.
    auto tx = 0.5 * (max[0] + min[0]);
    auto ty = 0.5 * (max[1] + min[1]);
    auto tz = 0.5 * (max[2] + min[2]);
    if ((tx == 0.0) && (ty == 0.0) && (tz == 0.0)) {
        return solid;
    } else {
        orphans.push_back(solid);
        auto translation = G4ThreeVector(tx, ty, tz);
        auto displaced = new G4DisplacedSolid(
            pathname,
            solid,
            nullptr,
            translation
        );
        return displaced;
    }
}

static G4TessellatedSolid * build_tessellation(
    const std::string & pathname,
    const Volume & volume
) {
    auto solid = new G4TessellatedSolid(pathname);
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
                pathname
            );
            set_error(ErrorType::ValueError, message.c_str());
            return nullptr;
        }
    }
    solid->SetSolidClosed(true);

    return solid;
}

static const G4VSolid * build_solids(
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

    // Build sub-solids.
    std::list<const G4VSolid *> daughters;
    for (auto && v: volume.volumes()) {
        auto s = build_solids(v, pathname, solids, orphans);
        if (s == nullptr) {
            return nullptr;
        } else {
            daughters.push_back(s);
        }
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

    // Build current solid.
    G4VSolid * solid = nullptr;
    switch (volume.shape()) {
        case ShapeType::Box: {
                auto shape = volume.box_shape();
                solid = new G4Box(
                    std::string(pathname),
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
                    std::string(pathname),
                    rmin * CLHEP::cm,
                    shape.radius * CLHEP::cm,
                    0.5 * shape.length * CLHEP::cm,
                    0.0,
                    CLHEP::twopi
                );
            }
            break;
        case ShapeType::Envelope:
            solid = build_envelope(pathname, volume, daughters, orphans);
            break;
        case ShapeType::Sphere: {
                auto shape = volume.sphere_shape();
                solid = new G4Orb(
                    std::string(pathname),
                    shape.radius * CLHEP::cm
                );
            }
            break;
        case ShapeType::Tessellation:
            solid = build_tessellation(pathname, volume);
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
        return nullptr;
    }
    solids[pathname] = solid;

    return solid;
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
    std::map<std::string, G4VSolid *> & solids
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
    auto logical = new G4LogicalVolume(solid, material, pathname);
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
        auto l = build_volumes(v, pathname, solids);
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
        new G4PVPlacement(
            rotation,
            position,
            l,
            v_path,
            logical,
            false,
            0
        );
    }

    return logical;
}

static void map_volumes(
    const G4VPhysicalVolume * self,
    std::map<std::string, const G4VPhysicalVolume *> & elements,
    std::map<const G4VPhysicalVolume *, const G4VPhysicalVolume *> & mothers
) {
    auto * logical = self->GetLogicalVolume();
    int n = logical->GetNoDaughters();
    for (int i = 0; i < n; i++) {
        auto daughter = logical->GetDaughter(i);
        elements[daughter->GetName()] = daughter;
        mothers[daughter] = self;
        map_volumes(daughter, elements, mothers);
    }
}

GeometryData::GeometryData(rust::Box<Volume> volume) {
    clear_error();
    this->world = nullptr;

    // Build solids.
    std::map<std::string, G4VSolid *> solids;
    const std::string path = "";
    if (build_solids(*volume, path, solids, this->orphans) == nullptr) {
        for (auto item: solids) {
            delete item.second;
        }
        return;
    }

    // Build volumes.
    auto logical = build_volumes(*volume, path, solids);
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
    this->mothers[this->world] = nullptr;
    this->INSTANCES[this->world] = this;

    // Map volumes hierarchy.
    map_volumes(this->world, this->elements, this->mothers);
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
    std::map<std::string, const G4VPhysicalVolume *> & elements,
    std::map<const G4VPhysicalVolume *, const G4VPhysicalVolume *> & mothers
) {
    auto transform = G4AffineTransform();
    if (volume == frame) {
        return std::make_pair(transform, nullptr);
    }

    const G4VPhysicalVolume * current = get_volume(volume, elements);
    const G4VPhysicalVolume * target = get_volume(frame, elements);
    if (any_error()) {
        return std::make_pair(transform, nullptr);
    }

    std::list<const G4VPhysicalVolume *> volumes;
    while (current != target) {
        volumes.push_back(current);
        current = mothers[current];
        if (current == nullptr) {
            auto msg = fmt::format(
                "'{}' does not contain '{}'",
                frame,
                volume
            );
            set_error(ErrorType::ValueError, msg.c_str());
            return std::make_pair(transform, nullptr);
        }
    }

    while (!volumes.empty()) {
        current = volumes.back();
        transform *= G4AffineTransform(
            current->GetRotation(),
            current->GetTranslation()
        );
        volumes.pop_back();
    }

    return std::make_pair(transform, current);
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

    auto result = get_transform(
        volume,
        frame,
        this->data->elements,
        this->data->mothers
    );
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

    auto result = get_transform(
        volume,
        frame,
        this->data->elements,
        this->data->mothers
    );
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
        auto mother = this->data->mothers[volume];
        if (mother == nullptr) {
            info.mother = rust::String("");
        } else {
            info.mother = rust::String(mother->GetName());
        }
        int n = logical->GetNoDaughters();
        for (int i = 0; i < n; i++) {
            auto daughter = logical->GetDaughter(i);
            info.daughters.push_back(
                std::move(std::string(daughter->GetName()
            )));
        }
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
