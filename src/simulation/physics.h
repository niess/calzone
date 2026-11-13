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


struct PhysicsSettings {
    double default_cut;
    EmPhysicsModel em_model;
    HadPhysicsModel had_model;
    bool auger;
    bool fluo;
    bool pixe;
};

struct PhysicsImpl: public G4VUserPhysicsList {
    PhysicsImpl(const PhysicsImpl &) = delete; // Forbid copy.

    // Geant4 interface.
    void ConstructParticle();
    void ConstructProcess();

    // User interface.
    void Update();
    PhysicsSettings & Settings() { return this->settings; }

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
    PhysicsSettings settings = {
        0.1,
        EmPhysicsModel::Standard,
        HadPhysicsModel::None,
        false,
        true,
        false
    };
};
