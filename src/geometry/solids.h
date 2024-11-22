#pragma once
// Geant4 interface.
#include "G4Box.hh"
#include "G4DisplacedSolid.hh"
#include "G4Orb.hh"
#include "G4Sphere.hh"
#include "G4SubtractionSolid.hh"
#include "G4Tubs.hh"
// Calzone interface.
#include "calzone.h"

struct Box: public G4Box {
    using G4Box::G4Box;

    G4VSolid * Clone() const override;
    G4ThreeVector GetPointOnSurface() const override;
};

struct DisplacedSolid: public G4DisplacedSolid {
    using G4DisplacedSolid::G4DisplacedSolid;

    G4VSolid * Clone() const override;
};

struct Orb: public G4Orb {
    using G4Orb::G4Orb;

    G4VSolid * Clone() const override;
};

struct Sphere: public G4Sphere {
    using G4Sphere::G4Sphere;

    G4VSolid * Clone() const override;
    G4ThreeVector GetPointOnSurface() const override;
};

struct SubtractionSolid: public G4SubtractionSolid {
    using G4SubtractionSolid::G4SubtractionSolid;

    G4VSolid * Clone() const override;
};

struct Tubs: public G4Tubs {
    using G4Tubs::G4Tubs;

    G4VSolid * Clone() const override;
    G4ThreeVector GetPointOnSurface() const override;
};
