#pragma once
// Geant4 interface.
#include "G4ParticleGun.hh"
#include "G4VUserPrimaryGeneratorAction.hh"
// User interface.
#include "calzone.h"


struct SourceImpl: G4VUserPrimaryGeneratorAction {
    SourceImpl(const SourceImpl &) = delete; // Forbid copy.

    // Geant4 interface.
    void GeneratePrimaries(G4Event *);

    // User interface.
    static SourceImpl * Get();

private:
    SourceImpl() = default;

    // Geant4 interface.
    G4ParticleGun gun = G4ParticleGun(1);
};
