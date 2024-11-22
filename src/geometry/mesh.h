#pragma once
// Geant4 interface.
#include "G4VSolid.hh"
// Calzone interface.
#include "calzone.h"

// Mesh wrapper.
struct Mesh: public G4VSolid {
    Mesh(const G4String &, const Volume &);
    Mesh(const Mesh &) = delete;

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

    const rust::Box<MeshHandle> & Describe() const;

private:
    rust::Box<MeshHandle> mesh;
};

// Geant4 TessellatedSolid wrapper.
struct TessellatedSolid: public G4VSolid {
    TessellatedSolid(const G4String &, const Volume &);
    TessellatedSolid(const Mesh &) = delete;

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

    const rust::Box<TessellatedSolidHandle> & Describe() const;

private:
    rust::Box<TessellatedSolidHandle> solid;
};
