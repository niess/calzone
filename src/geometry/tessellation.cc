#include "tessellation.h"
// Geant4 interface.
#include "G4AffineTransform.hh"
#include "G4BoundingEnvelope.hh"
#include "Randomize.hh"


TessellatedSolid::TessellatedSolid(
    const G4String & name,
    const TessellatedShape & shape
):
    G4VSolid::G4VSolid(name),
    tessels(sort_tessels(shape))
{}

void TessellatedSolid::BoundingLimits(
    G4ThreeVector & pMin,
    G4ThreeVector & pMax) const {
    auto && envelope = this->tessels->envelope();
    pMin[0] = envelope[0][0];
    pMin[1] = envelope[0][1];
    pMin[2] = envelope[0][2];
    pMax[0] = envelope[1][0];
    pMax[1] = envelope[1][1];
    pMax[2] = envelope[1][2];
}

G4bool TessellatedSolid::CalculateExtent(
    const EAxis axis,
    const G4VoxelLimits & limits,
    const G4AffineTransform & transform,
    G4double & min,
    G4double & max
) const {
    auto && envelope = this->tessels->envelope();
    G4ThreeVector bmin, bmax;
    bmin[0] = envelope[0][0], bmax[0] = envelope[1][0];
    bmin[1] = envelope[0][1], bmax[1] = envelope[1][1];
    bmin[2] = envelope[0][2], bmax[2] = envelope[1][2];

    G4BoundingEnvelope bbox(bmin, bmax);
    return bbox.CalculateExtent(axis, limits, transform, min, max);
}

G4double TessellatedSolid::DistanceToIn(const G4ThreeVector & position) const {
    auto && envelope = this->tessels->envelope();
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

G4double TessellatedSolid::DistanceToIn(
    const G4ThreeVector & position, const G4ThreeVector & direction
) const {
    auto && distance = this->tessels->distance_to_in(position, direction);
    const double delta = 0.5 * kCarTolerance;
    if ((distance <= delta) || (distance > kInfinity)) {
        return kInfinity;
    } else {
        return distance;
    }
}

G4double TessellatedSolid::DistanceToOut(const G4ThreeVector &) const {
    return 0.0;
}

G4double TessellatedSolid::DistanceToOut(
    const G4ThreeVector & position,
    const G4ThreeVector & direction,
    G4bool calculateNormal,
    G4bool * validNormal,
    G4ThreeVector * normal
) const {
    long index;
    auto && distance = this->tessels->distance_to_out(
        position, direction, index
    );
    if (calculateNormal) {
        if (index >= 0) {
            *validNormal = true;
            auto && n = this->tessels->normal(index);
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

G4GeometryType TessellatedSolid::GetEntityType() const {
    return { "TessellatedSolid" };
}

G4ThreeVector TessellatedSolid::GetPointOnSurface () const {
    auto && point = this->tessels->surface_point(
        G4UniformRand(),
        G4UniformRand(),
        G4UniformRand()
    );
    return G4ThreeVector(point[0], point[1], point[2]);
}

G4double TessellatedSolid::GetSurfaceArea() {
    return this->tessels->area();
}

EInside TessellatedSolid::Inside(const G4ThreeVector & position) const {
    const double delta = 0.5 * kCarTolerance;
    return this->tessels->inside(position, delta);
}

G4ThreeVector TessellatedSolid::SurfaceNormal(
    const G4ThreeVector & position
) const {
    auto && normal = this->tessels->surface_normal(position);
    return G4ThreeVector(normal[0], normal[1], normal[2]);
}

void TessellatedSolid::DescribeYourselfTo(G4VGraphicsScene &) const {}

std::ostream & TessellatedSolid::StreamInfo(std::ostream & stream) const {
    return stream;
}