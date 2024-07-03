// Geant4 interface.
#include "G4VSensitiveDetector.hh"
// User interface.
#include "calzone.h"

struct SamplerImpl : public G4VSensitiveDetector {
    SamplerImpl(const std::string &, Roles);
    SamplerImpl(const SamplerImpl &) = delete;

    // Geant4 interface.
    G4bool ProcessHits(G4Step *, G4TouchableHistory *);

    // User interface.
    Roles roles;
};
