#pragma once
// Geant4 interface.
#include "G4DecayPhysics.hh"
#include "G4EmExtraPhysics.hh"
#include "G4IonPhysics.hh"
#include "G4HadronElasticPhysics.hh"
#include "G4StoppingPhysics.hh"
#include "G4VUserPhysicsList.hh"
// User interface.
#include "calzone.h"


struct PhysicsImpl: public G4VUserPhysicsList {
    PhysicsImpl(const PhysicsImpl &) = delete; // Forbid copy.

    // Geant4 interface.
    void ConstructParticle();
    void ConstructProcess();

    // User interface.
    void DisableVerbosity() const;
    void Update();

    static PhysicsImpl * Get();

private:
    PhysicsImpl() = default;

    // Geant4 interface.
    std::unique_ptr<G4DecayPhysics> decayPhysics = nullptr;
    std::unique_ptr<G4VPhysicsConstructor> emPhysics = nullptr;
    std::unique_ptr<G4EmExtraPhysics> extraPhysics = nullptr;
    std::unique_ptr<G4VPhysicsConstructor> hadPhysics = nullptr;
    std::unique_ptr<G4HadronElasticPhysics> hePhysics = nullptr;
    std::unique_ptr<G4StoppingPhysics> stoppingPhysics = nullptr;
    std::unique_ptr<G4IonPhysics> ionPhysics = nullptr;

    // User interface.
    EmPhysicsModel current_em_model = EmPhysicsModel::None;
    HadPhysicsModel current_had_model = HadPhysicsModel::None;
};
