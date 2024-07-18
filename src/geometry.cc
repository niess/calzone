#include "calzone.h"
#include "geometry/solids.h"
#include "geometry/tessellation.h"
#include "simulation/sampler.h"
// standard library.
#include <list>
// fmt library.
#include <fmt/core.h>
// Geant4 interface.
#include "G4GDMLParser.hh"
#include "G4NistManager.hh"
#include "G4PVPlacement.hh"
#include "G4SmartVoxelHeader.hh"
#include "G4TriangularFacet.hh"
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
    GeometryData(const rust::Box<Volume> &, const TSTAlgorithm &);
    ~GeometryData();

    GeometryData(const GeometryData &) = delete; // Forbid copy.

    GeometryData * clone();
    void drop();

    static GeometryData * get(const G4VPhysicalVolume *);

    size_t id = 0;
    G4VPhysicalVolume * world = nullptr;
    std::map<std::string, const G4VPhysicalVolume *> elements;
    std::map<const G4VPhysicalVolume *, const G4VPhysicalVolume *> mothers;

private:
    size_t rc = 0;
    std::list <const G4VSolid *> orphans;

    static size_t LAST_ID;
    static std::map<const G4VPhysicalVolume *, GeometryData *> INSTANCES;
};

size_t GeometryData::LAST_ID = 0;
std::map<const G4VPhysicalVolume *, GeometryData *> GeometryData::INSTANCES;

static G4AffineTransform local_transform(const Volume & v) {
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
        return std::move(G4AffineTransform(rotation, translation));
    } else {
        return std::move(G4AffineTransform(translation));
    }
}

static G4VSolid * build_envelope(
    const std::string & pathname,
    const Volume & volume,
    std::list<const G4VSolid *> & daughters,
    std::list<const G4VSolid *> & orphans
) {
    // Compute limits along X, Y and Z axis.
    auto envelope = volume.envelope_shape();
    std::array<double, 3> min = { DBL_MAX, DBL_MAX, DBL_MAX };
    std::array<double, 3> max = { -DBL_MAX, -DBL_MAX, -DBL_MAX };
    for (auto && v: volume.volumes()) {
        std::array<double, 3> mi;
        std::array<double, 3> mx;
        const G4VSolid * s = daughters.front();
        daughters.pop_front();
        auto t = local_transform(v);
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
            solid = new Box(
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
                solid = new Tubs(
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
                solid = new Orb(
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
        auto displaced = new DisplacedSolid(
            pathname,
            solid,
            nullptr,
            translation
        );
        return displaced;
    }
}

static TessellatedSolid * build_geant4_tessellation(
    const std::string & pathname,
    const Volume & volume
) {
    auto solid = new TessellatedSolid(pathname);
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

static G4VSolid * build_tessellation(
    const TSTAlgorithm & algorithm,
    const std::string & pathname,
    const Volume & volume
) {
    switch (algorithm) {
        case TSTAlgorithm::Bvh: {
            auto && shape = volume.tessellated_shape();
            return new Tessellation(pathname, shape);
        }
        case TSTAlgorithm::Geant4:
            return build_geant4_tessellation(pathname, volume);
        default:
            return nullptr; // unreachable.
    }
}

static G4VSolid * build_solids(
    const Volume & volume,
    const TSTAlgorithm & algorithm,
    const std::string & path,
    std::map<std::string, G4VSolid *> & solids,
    std::list<const G4VSolid *> & orphans
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
    std::map<rust::String, G4AffineTransform> transforms;
    std::list<std::array<rust::String, 2>> subtractions;
    for (auto && v: volume.volumes()) {
        auto s = build_solids(v, algorithm, pathname, solids, orphans);
        if (s == nullptr) {
            return nullptr;
        } else {
            daughters.push_back(s);
            auto && t = local_transform(v);
            transforms[v.name()] = std::move(t);

            for (auto && subtract: v.subtract()) {
                std::array<rust::String, 2> item = {
                    v.name(),
                    subtract
                };
                subtractions.push_back(std::move(item));
            }
        }
    }

    // Apply subtractions and overlaps.
    auto subtract = [&](const std::array<rust::String, 2> & item) {
        const std::string path0 = fmt::format("{}.{}",
            pathname, std::string(item[0]));
        const std::string path1 = fmt::format("{}.{}",
            pathname, std::string(item[1]));
        auto solid0 = solids[path0];
        auto && t0 = transforms[item[0]];
        auto && t1 = transforms[item[1]];
        SubtractionSolid * boolean;
        if (t1.IsTranslated() || t1.IsRotated()) {
            if (t0.IsTranslated() || t0.IsRotated()) {
                boolean = new SubtractionSolid(
                    std::string(item[0]),
                    solid0,
                    solids[path1],
                    t0 * t1.Inverse() // XXX Check this.
                );
            } else {
                boolean = new SubtractionSolid(
                    std::string(item[0]),
                    solid0,
                    solids[path1],
                    t1.Inverse()
                );
            }
        } else {
            if (t0.IsTranslated() || t0.IsRotated()) {
                auto t = t0 * t1.Inverse();
                boolean = new SubtractionSolid(
                    std::string(item[0]),
                    solid0,
                    solids[path1],
                    t0
                );
            } else {
                boolean = new SubtractionSolid(
                    std::string(item[0]),
                    solid0,
                    solids[path1]
                );
            }
        }
        orphans.push_back(solid0);
        solids[path0] = boolean;
    };

    for (auto overlap: volume.overlaps()) {
        subtract(overlap);
    }

    for (auto item: subtractions) {
        subtract(item);
    }

    // Build current solid.
    G4VSolid * solid = nullptr;
    switch (volume.shape()) {
        case ShapeType::Box: {
                auto shape = volume.box_shape();
                solid = new Box(
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
                double phi0 = (shape.section[0] / 360.0) * CLHEP::twopi;
                double dphi = ((shape.section[1] - shape.section[0]) / 360.0)
                    * CLHEP::twopi;
                solid = new Tubs(
                    std::string(pathname),
                    rmin * CLHEP::cm,
                    shape.radius * CLHEP::cm,
                    0.5 * shape.length * CLHEP::cm,
                    phi0,
                    dphi
                );
            }
            break;
        case ShapeType::Envelope:
            solid = build_envelope(pathname, volume, daughters, orphans);
            break;
        case ShapeType::Sphere: {
                auto shape = volume.sphere_shape();
                if ((shape.thickness <= 0.0) &&
                    (shape.azimuth_section[0] == 0.0) &&
                    (shape.azimuth_section[1] == 360.0) &&
                    (shape.zenith_section[0] == 0.0) &&
                    (shape.zenith_section[1] == 180.0)) {
                    solid = new Orb(
                        std::string(pathname),
                        shape.radius * CLHEP::cm
                    );
                } else {
                    double rmin = (shape.thickness > 0.0) ?
                        shape.radius - shape.thickness : 0.0;
                    double phi0 = (shape.azimuth_section[0] / 360.0) *
                        CLHEP::twopi;
                    double dphi = ((shape.azimuth_section[1] -
                        shape.azimuth_section[0]) / 360.0) * CLHEP::twopi;
                    double theta0 = (shape.zenith_section[0] / 180.0) *
                        CLHEP::pi;
                    double dtheta = ((shape.zenith_section[1] -
                        shape.zenith_section[0]) / 180.0) * CLHEP::pi;
                    solid = new Sphere(
                        std::string(pathname),
                        shape.radius * CLHEP::cm,
                        rmin * CLHEP::cm,
                        phi0,
                        dphi,
                        theta0,
                        dtheta
                    );
                }
            }
            break;
        case ShapeType::Tessellation:
            solid = build_tessellation(algorithm, pathname, volume);
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
    delete logical->GetVoxelHeader();
    logical->SetVoxelHeader(nullptr);
    delete logical->GetSolid();
    delete logical->GetSensitiveDetector();
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

    // Set any sensitive detector.
    if (volume.sensitive()) {
        auto sampler = new SamplerImpl(pathname, std::move(volume.roles()));
        logical->SetSensitiveDetector(sampler);
    }

    // Build sub-volumes.
    for (auto && v: volume.volumes()) {
        auto l = build_volumes(v, pathname, solids);
        if (l == nullptr) {
            drop_them_all(logical);
            return nullptr;
        }

        auto && p = v.position();
        auto position = G4ThreeVector(
            p[0] * CLHEP::cm,
            p[1] * CLHEP::cm,
            p[2] * CLHEP::cm
        );
        G4RotationMatrix * rotation = nullptr;
        if (v.is_rotated()) {
            auto && m = v.rotation();
            auto rowX = G4ThreeVector(m[0][0], m[0][1], m[0][2]);
            auto rowY = G4ThreeVector(m[1][0], m[1][1], m[1][2]);
            auto rowZ = G4ThreeVector(m[2][0], m[2][1], m[2][2]);
            rotation = new G4RotationMatrix();
            rotation->setRows(rowX, rowY, rowZ);
        }
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

GeometryData::GeometryData(
    const rust::Box<Volume> & volume,
    const TSTAlgorithm & algorithm
) {
    clear_error();
    this->id = ++GeometryData::LAST_ID;
    this->world = nullptr;

    // Build solids.
    std::map<std::string, G4VSolid *> solids;
    const std::string path = "";
    auto top_solid = build_solids(
        *volume, algorithm, path, solids, this->orphans
    );
    if (top_solid == nullptr) {
        for (auto item: solids) {
            delete item.second;
        }
        return;
    }

    // Displace top solid (if requested).
    if ((volume->is_translated() || volume->is_rotated())) {
        auto && p = volume->position();
        auto position = G4ThreeVector(
            p[0] * CLHEP::cm,
            p[1] * CLHEP::cm,
            p[2] * CLHEP::cm
        );
        G4RotationMatrix * rotation = nullptr;
        if (volume->is_rotated()) {
            auto && m = volume->rotation();
            auto rowX = G4ThreeVector(m[0][0], m[0][1], m[0][2]);
            auto rowY = G4ThreeVector(m[1][0], m[1][1], m[1][2]);
            auto rowZ = G4ThreeVector(m[2][0], m[2][1], m[2][2]);
            rotation = new G4RotationMatrix();
            rotation->setRows(rowX, rowY, rowZ);
        }
        this->orphans.push_back(top_solid);
        auto name = std::string(volume->name());
        top_solid = new DisplacedSolid(
            name,
            top_solid,
            rotation,
            position
        );
        solids[name] = top_solid;
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

std::shared_ptr<GeometryBorrow> create_geometry(
    const rust::Box<Volume> & volume,
    const TSTAlgorithm & algorithm
) {
    auto data = new GeometryData(volume, algorithm);
    if (any_error()) {
        delete data;
        return nullptr;
    } else {
        return std::make_shared<GeometryBorrow>(data);
    }
}

static const G4VPhysicalVolume * get_volume(
    const std::string & path,
    std::map<std::string, const G4VPhysicalVolume *> & elements
) {
    auto volume = elements[path];
    if (volume == nullptr) {
        auto msg = fmt::format("unknown volume '{}'", path);
        set_error(ErrorType::ValueError, msg.c_str());
    }
    return volume;
}

std::shared_ptr<VolumeBorrow> GeometryBorrow::borrow_volume(
    rust::Str name_
) const {
    auto name = std::string(name_);
    auto volume = get_volume(name, this->data->elements);
    if (volume == nullptr) {
        return nullptr;
    } else {
        return std::make_shared<VolumeBorrow>(this->data, volume);
    }
}


// ============================================================================
//
// Geant4 interface.
//
// ============================================================================

static void check_overlaps(G4VPhysicalVolume * volume, int resolution) {
    volume->CheckOverlaps(resolution, DBL_EPSILON, false);
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

std::shared_ptr<Error> GeometryBorrow::dump(rust::Str path) const {
    clear_error();
    G4GDMLParser parser;
    auto buffer = std::cout.rdbuf();
    std::cout.rdbuf(nullptr); // Disable cout temporarly.
    parser.Write(std::string(path), this->data->world);
    std::cout.rdbuf(buffer);
    return get_error();
}

size_t GeometryBorrow::id() const {
    return this->data->id;
}

G4VPhysicalVolume * GeometryBorrow::world() const {
    return this->data->world;
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

static void optimise(G4VPhysicalVolume * physical) {
    const int MIN_VOXEL_VOLUMES_LEVEL_1 = 2; // from voxeldefs.hh.

    auto && volume = physical->GetLogicalVolume();
    size_t n = volume->GetNoDaughters();

    auto && head = volume->GetVoxelHeader();
    if (head == nullptr) {
        if ((volume->IsToOptimise() && (n >= MIN_VOXEL_VOLUMES_LEVEL_1)) ||
            ((n == 1) && (volume->GetDaughter(0)->IsReplicated()))) {
            auto && head = new G4SmartVoxelHeader(volume);
            volume->SetVoxelHeader(head);
        }
    }

    for (size_t i = 0; i < n; i++) {
        optimise(volume->GetDaughter(i));
    }
}

const G4VPhysicalVolume * G4Goupil::NewGeometry() {
    auto geometry = GOUPIL_GEOMETRY->clone();
    optimise(geometry->world);
    return geometry->world;
}

void G4Goupil::DropGeometry(const G4VPhysicalVolume * volume) {
    auto geometry = GeometryData::get(volume);
    geometry->drop();
}


// ============================================================================
//
// Volume interface.
//
// ============================================================================

VolumeBorrow::VolumeBorrow(GeometryData * g, const G4VPhysicalVolume * v):
    geometry(g->clone()), volume(v) {};

VolumeBorrow::~VolumeBorrow() {
    this->geometry->drop();
}

std::array<double, 6> VolumeBorrow::compute_box(rust::Str frame) const {
    std::array<double, 6> box = { 0.0, 0.0, 0.0, 0.0, 0.0, 0.0 };
    auto transform = this->compute_transform(frame);
    if (any_error()) {
        return box;
    }

    auto solid = this->volume->GetLogicalVolume()->GetSolid();
    if (transform->IsTranslated() || transform->IsRotated()) {
        auto limits = G4VoxelLimits();
        solid->CalculateExtent(kXAxis, limits, *transform, box[0], box[1]);
        solid->CalculateExtent(kYAxis, limits, *transform, box[2], box[3]);
        solid->CalculateExtent(kZAxis, limits, *transform, box[4], box[5]);
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

std::unique_ptr<G4AffineTransform> VolumeBorrow::compute_transform(
    rust::Str frame_
) const {
    std::string frame;
    if (frame_.empty()) {
        frame = this->geometry->world->GetName();
    } else {
        frame = std::string(frame_);
    }
    auto transform = std::make_unique<G4AffineTransform>();
    auto && volume = this->volume->GetName();
    if (volume == frame) {
        return transform;
    }

    const G4VPhysicalVolume * current = this->volume;
    const G4VPhysicalVolume * target = get_volume(
        frame,
        this->geometry->elements
    );
    if (any_error()) {
        return nullptr;
    }

    std::list<const G4VPhysicalVolume *> volumes;
    while (current != target) {
        volumes.push_back(current);
        current = this->geometry->mothers[current];
        if (current == nullptr) {
            auto msg = fmt::format(
                "'{}' does not contain '{}'",
                frame,
                volume
            );
            set_error(ErrorType::ValueError, msg.c_str());
            return nullptr;
        }
    }

    while (!volumes.empty()) {
        current = volumes.back();
        *transform *= G4AffineTransform(
            current->GetRotation(),
            current->GetTranslation()
        );
        volumes.pop_back();
    }

    return transform;
}

std::array<double, 3> VolumeBorrow::compute_origin(rust::Str frame) const {
    std::array<double, 3> origin = { 0.0, 0.0, 0.0 };
    auto transform = this->compute_transform(frame);
    if (any_error()) {
        return origin;
    }

    auto p = transform->TransformPoint(G4ThreeVector(0.0, 0.0, 0.0));
    for (auto i = 0; i < 3; i++) {
        origin[i] = p[i] / CLHEP::cm;
    }

    return origin;
}

double VolumeBorrow::compute_surface() const {
    auto && solid = this->volume->GetLogicalVolume()->GetSolid();
    return solid->GetSurfaceArea() / CLHEP::cm2;
}

double VolumeBorrow::compute_volume(bool include_daughters) const {
    auto && logical = this->volume->GetLogicalVolume();
    auto volume = logical->GetSolid()->GetCubicVolume();
    if (!include_daughters) {
        size_t n = logical->GetNoDaughters();
        for (size_t i = 0; i < n; i++) {
            auto && daughter = logical->GetDaughter(i);
            volume -= daughter
                -> GetLogicalVolume()
                -> GetSolid()
                -> GetCubicVolume();
        }
    }
    return std::max(volume, 0.0) / CLHEP::cm3;
}

VolumeInfo VolumeBorrow::describe() const {
    VolumeInfo info;
    auto logical = this->volume->GetLogicalVolume();
    info.material = rust::String(logical->GetMaterial()->GetName());
    info.solid = rust::String(logical->GetSolid()->GetEntityType());
    auto mother = this->geometry->mothers[this->volume];
    if (mother == nullptr) {
        info.mother = rust::String("");
    } else {
        info.mother = rust::String(mother->GetName());
    }
    int n = logical->GetNoDaughters();
    for (int i = 0; i < n; i++) {
        auto daughter = logical->GetDaughter(i);
        info.daughters.push_back({
            std::move(std::string(daughter->GetName())),
            std::move(std::string(
                daughter->GetLogicalVolume()->GetSolid()->GetEntityType()
            )),
        });
    }
    return info;
}

std::array<double, 6> VolumeBorrow::generate_onto(
    RandomContext &, // Implicit scope.
    const G4AffineTransform & transform,
    bool compute_normal
) const {
    auto && solid = this->volume->GetLogicalVolume()->GetSolid();
    auto && point = solid->GetPointOnSurface();
    G4ThreeVector normal;
    if (compute_normal) {
        normal = solid->SurfaceNormal(point);
    }
    if (transform.IsRotated() || transform.IsTranslated()) {
        point = transform.TransformPoint(point);
        normal = transform.TransformAxis(normal);
    }
    std::array<double, 6> result = {
        point.x() / CLHEP::cm,
        point.y() / CLHEP::cm,
        point.z() / CLHEP::cm,
        normal.x(),
        normal.y(),
        normal.z()
    };
    return result;
}

EInside VolumeBorrow::inside(
    const std::array<double, 3> & point_,
    const G4AffineTransform & transform,
    bool include_daughters
) const {
    G4ThreeVector point(
        point_[0] * CLHEP::cm,
        point_[1] * CLHEP::cm,
        point_[2] * CLHEP::cm
    );
    if (transform.IsTranslated() || transform.IsRotated()) {
        point = transform.InverseTransformPoint(point);
    }
    auto && solid = this->volume->GetLogicalVolume()->GetSolid();
    auto inside = solid->Inside(point);
    if ((include_daughters == true) || (inside != EInside::kInside)) {
        return inside;
    }

    auto && logical = this->volume->GetLogicalVolume();
    size_t n = logical->GetNoDaughters();
    for (size_t i = 0; i < n; i++) {
        auto && daughter = logical->GetDaughter(i);
        auto && translation = daughter->GetTranslation();
        auto && rotation = daughter->GetRotation();
        G4AffineTransform t;
        if (rotation == nullptr) {
            t = G4AffineTransform(translation);
        } else {
            t = G4AffineTransform(rotation, translation);
        }
        G4ThreeVector ri;
        if (t.IsTranslated() || t.IsRotated()) {
            ri = t.InverseTransformPoint(point);
        } else {
            ri = point;
        }
        auto si = daughter->GetLogicalVolume()->GetSolid()->Inside(ri);
        switch (si) {
            case EInside::kSurface:
                return EInside::kSurface;
            case EInside::kInside:
                return EInside::kOutside;
            default:
                continue;
        }
    }
    return EInside::kInside;
}


// ============================================================================
//
// Volume roles interface.
//
// ============================================================================

void VolumeBorrow::clear_roles() const {
    auto && logical = this->volume->GetLogicalVolume();
    SamplerImpl * sensitive = static_cast<SamplerImpl *>(
        logical->GetSensitiveDetector()
    );
    if (sensitive != nullptr) {
        logical->SetSensitiveDetector(nullptr);
        delete sensitive;
    }
}

Roles VolumeBorrow::get_roles() const {
    auto && logical = this->volume->GetLogicalVolume();
    auto sensitive = static_cast<SamplerImpl *>(
        logical->GetSensitiveDetector()
    );
    if (sensitive == nullptr) {
        Roles roles;
        std::memset(&roles, 0x0, sizeof(Roles));
        return roles;
    } else {
        return sensitive->roles;
    }
}

void VolumeBorrow::set_roles(Roles roles) const {
    auto && logical = this->volume->GetLogicalVolume();
    SamplerImpl * sensitive = static_cast<SamplerImpl *>(
        logical->GetSensitiveDetector()
    );
    if (sensitive == nullptr) {
        sensitive = new SamplerImpl(logical->GetName(), std::move(roles));
        logical->SetSensitiveDetector(sensitive);
    } else {
        sensitive->roles = std::move(roles);
    }
}
