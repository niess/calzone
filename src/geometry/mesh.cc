#include "mesh.h"
// Geant4 interface.
#include "G4AffineTransform.hh"
#include "G4BoundingEnvelope.hh"
#include "Randomize.hh"


// ============================================================================
//
// Mesh wrapper
//
// ============================================================================

Mesh::Mesh(
    const G4String & name,
    const Volume & volume
):
    G4VSolid::G4VSolid(name),
    mesh(volume.get_mesh())
{}

void Mesh::BoundingLimits(
    G4ThreeVector & pMin,
    G4ThreeVector & pMax) const {
    auto && envelope = this->mesh->envelope();
    pMin[0] = envelope[0][0];
    pMin[1] = envelope[0][1];
    pMin[2] = envelope[0][2];
    pMax[0] = envelope[1][0];
    pMax[1] = envelope[1][1];
    pMax[2] = envelope[1][2];
}

G4bool Mesh::CalculateExtent(
    const EAxis axis,
    const G4VoxelLimits & limits,
    const G4AffineTransform & transform,
    G4double & min,
    G4double & max
) const {
    auto && envelope = this->mesh->envelope();
    G4ThreeVector bmin, bmax;
    bmin[0] = envelope[0][0], bmax[0] = envelope[1][0];
    bmin[1] = envelope[0][1], bmax[1] = envelope[1][1];
    bmin[2] = envelope[0][2], bmax[2] = envelope[1][2];

    G4BoundingEnvelope bbox(bmin, bmax);
    return bbox.CalculateExtent(axis, limits, transform, min, max);
}

G4double Mesh::DistanceToIn(const G4ThreeVector & position) const {
    auto && envelope = this->mesh->envelope();
    G4ThreeVector center(
        0.5 * (envelope[0][0] + envelope[1][0]),
        0.5 * (envelope[0][1] + envelope[1][1]),
        0.5 * (envelope[0][2] + envelope[1][2])
    );
    G4ThreeVector hw(
        0.5 * std::abs(envelope[0][0] - envelope[1][0]),
        0.5 * std::abs(envelope[0][1] - envelope[1][1]),
        0.5 * std::abs(envelope[0][2] - envelope[1][2])
    );
    G4ThreeVector r = position - center;
    auto distance = std::max(std::max(
        std::abs(r.x()) - hw.x(),
        std::abs(r.y()) - hw.y()),
        std::abs(r.z()) - hw.z());

    const double delta = 0.5 * kCarTolerance;
    if (distance < delta) {
        return 0.0;
    } else if (distance > kInfinity) {
        return kInfinity;
    } else {
        return distance;
    }
}

G4double Mesh::DistanceToIn(
    const G4ThreeVector & position, const G4ThreeVector & direction
) const {
    auto && distance = this->mesh->distance_to_in(position, direction);
    const double delta = 0.5 * kCarTolerance;
    if ((distance <= delta) || (distance > kInfinity)) {
        return kInfinity;
    } else {
        return distance;
    }
}

G4double Mesh::DistanceToOut(const G4ThreeVector &) const {
    return 0.0;
}

G4double Mesh::DistanceToOut(
    const G4ThreeVector & position,
    const G4ThreeVector & direction,
    G4bool calculateNormal,
    G4bool * validNormal,
    G4ThreeVector * normal
) const {
    long index;
    auto && distance = this->mesh->distance_to_out(
        position, direction, index
    );
    if (calculateNormal) {
        if (index >= 0) {
            *validNormal = true;
            auto && n = this->mesh->normal(index);
            (*normal)[0] = n[0];
            (*normal)[1] = n[1];
            (*normal)[2] = n[2];
        } else {
            *validNormal = false;
        }
    }
    const double delta = 0.5 * kCarTolerance;
    if ((distance < delta) || (distance >= kInfinity)) {
        return 0.0;
    } else {
        return distance;
    }
}

G4GeometryType Mesh::GetEntityType() const {
    return { "Mesh" };
}

G4ThreeVector Mesh::GetPointOnSurface () const {
    auto && point = this->mesh->surface_point(
        G4UniformRand(),
        G4UniformRand(),
        G4UniformRand()
    );
    return G4ThreeVector(point[0], point[1], point[2]);
}

G4double Mesh::GetSurfaceArea() {
    return this->mesh->area();
}

EInside Mesh::Inside(const G4ThreeVector & position) const {
    const double delta = 0.5 * kCarTolerance;
    return this->mesh->inside(position, delta);
}

G4ThreeVector Mesh::SurfaceNormal(
    const G4ThreeVector & position
) const {
    const double delta = 0.5 * kCarTolerance;
    auto && normal = this->mesh->surface_normal(position, delta);
    return G4ThreeVector(normal[0], normal[1], normal[2]);
}

void Mesh::DescribeYourselfTo(G4VGraphicsScene &) const {}

std::ostream & Mesh::StreamInfo(std::ostream & stream) const {
    return stream;
}

const rust::Box<MeshHandle> & Mesh::Describe() const {
    return this->mesh;
}


// ============================================================================
//
// Geant4 TessellatedSolid wrapper
//
// ============================================================================

TessellatedSolid::TessellatedSolid(
    const G4String & name,
    const Volume & volume
):
    G4VSolid::G4VSolid(name),
    solid(volume.get_tessellated_solid())
{}

void TessellatedSolid::BoundingLimits(
    G4ThreeVector & pMin,
    G4ThreeVector & pMax
) const {
    return this->solid->ptr()->BoundingLimits(pMin, pMax);
}

G4bool TessellatedSolid::CalculateExtent(
    const EAxis axis,
    const G4VoxelLimits & limits,
    const G4AffineTransform & transform,
    G4double & min,
    G4double & max
) const {
    return this->solid->ptr()->CalculateExtent(
        axis,
        limits,
        transform,
        min,
        max
    );
}

G4double TessellatedSolid::DistanceToIn(const G4ThreeVector & position) const {
    return this->solid->ptr()->DistanceToIn(position);
}

G4double TessellatedSolid::DistanceToIn(
    const G4ThreeVector & position, const G4ThreeVector & direction
) const {
    return this->solid->ptr()->DistanceToIn(position, direction);
}

G4double TessellatedSolid::DistanceToOut(const G4ThreeVector & position) const {
    return this->solid->ptr()->DistanceToOut(position);
}

G4double TessellatedSolid::DistanceToOut(
    const G4ThreeVector & position,
    const G4ThreeVector & direction,
    G4bool calculateNormal,
    G4bool * validNormal,
    G4ThreeVector * normal
) const {
    return this->solid->ptr()->DistanceToOut(
        position,
        direction,
        calculateNormal,
        validNormal,
        normal
    );
}

G4GeometryType TessellatedSolid::GetEntityType() const {
    return { "G4TessellatedSolid" };
}

G4ThreeVector TessellatedSolid::GetPointOnSurface () const {
    auto ptr = this->solid->ptr();
    const int n = ptr->GetNumberOfFacets();
    if (n <= 0) {
        return ptr->GetPointOnSurface();
    }

    // Select facets according to their respective areas (contrary to Geant4
    // native implementation).
    const double target = ptr->GetSurfaceArea() * G4UniformRand();
    double area = 0.0;
    G4VFacet * facet = nullptr;
    for (int i = 0; i < n; i++) {
        facet = ptr->GetFacet(i);
        area += facet->GetArea();
        if (target <= area) {
            break;
        }
    }
    return facet->GetPointOnFace();
}

G4double TessellatedSolid::GetSurfaceArea() {
    return this->solid->ptr()->GetSurfaceArea();
}

EInside TessellatedSolid::Inside(const G4ThreeVector & position) const {
    return this->solid->ptr()->Inside(position);
}

G4ThreeVector TessellatedSolid::SurfaceNormal(
    const G4ThreeVector & position
) const {
    return this->solid->ptr()->SurfaceNormal(position);
}

void TessellatedSolid::DescribeYourselfTo(G4VGraphicsScene & scene) const {
    this->solid->ptr()->DescribeYourselfTo(scene);
}

std::ostream & TessellatedSolid::StreamInfo(std::ostream & stream) const {
    return this->solid->ptr()->StreamInfo(stream);
}

const rust::Box<TessellatedSolidHandle> & TessellatedSolid::Describe() const {
    return this->solid;
}
