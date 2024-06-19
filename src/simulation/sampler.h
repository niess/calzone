#include "G4VSensitiveDetector.hh"

struct SamplerImpl : public G4VSensitiveDetector {
    SamplerImpl(const SamplerImpl &) = delete;

    // Geant4 interface.
    G4bool ProcessHits(G4Step *, G4TouchableHistory *);

    // User interface.
    static SamplerImpl * Get();

private:
    SamplerImpl();

    std::map<std::string, std::string> processes;
};
