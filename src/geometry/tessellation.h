#pragma once
// Geant4 interface.
#include "G4VSolid.hh"
// Calzone interface.
#include "calzone.h"

struct Tessellation: public G4VSolid {
    Tessellation(const G4String &, const TessellatedShape & shape);
    Tessellation(const Tessellation &) = delete;

    void BoundingLimits(G4ThreeVector &, G4ThreeVector &) const;
    G4bool CalculateExtent(
        const EAxis,
        const G4VoxelLimits &,
        const G4AffineTransform &,
        G4double &,
        G4double &
    ) const;

    G4double DistanceToIn(const G4ThreeVector &) const;
    G4double DistanceToIn(const G4ThreeVector &, const G4ThreeVector &) const;
    G4double DistanceToOut(const G4ThreeVector &) const;
    G4double DistanceToOut(
        const G4ThreeVector &,
        const G4ThreeVector &,
        G4bool,
        G4bool *,
        G4ThreeVector *
    ) const;
    EInside Inside(const G4ThreeVector &) const;
    G4ThreeVector SurfaceNormal(const G4ThreeVector &) const;

    G4GeometryType GetEntityType() const;
    G4ThreeVector GetPointOnSurface () const;
    G4double GetSurfaceArea();

    void DescribeYourselfTo(G4VGraphicsScene &) const;
    std::ostream & StreamInfo(std::ostream &) const;

    const rust::Box<SortedTessels> & Describe() const;

private:
    rust::Box<SortedTessels> tessels;
};
