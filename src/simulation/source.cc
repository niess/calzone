// fmt library.
#include <fmt/core.h>
// User interface.
#include "random.h"
#include "source.h"
// Geant4 interafce.
#include "G4Event.hh"
#include "G4Geantino.hh"
#include "G4ParticleTable.hh"
#include "G4RunManager.hh"


void SourceImpl::GeneratePrimaries(G4Event * event) {
    if (RUN_AGENT->is_random_indices()) {
        RandomImpl::Get()->SetIndex(RUN_AGENT->next_random_index());
    }
    auto random_index = RandomImpl::Get()->GetIndex();
    auto primary = RUN_AGENT->next_primary(random_index);
    G4ParticleDefinition * definition;
    if (primary.pid != 0) {
        definition = G4ParticleTable::GetParticleTable()->FindParticle(
            primary.pid
        );
    } else {
        definition = G4Geantino::Definition();
    }
    if (definition == nullptr) {
        event->SetEventAborted();
        auto manager = G4RunManager::GetRunManager();
        manager->AbortRun(true);
        auto msg = fmt::format(
            "bad pid (expected a valid PDG encoding, found '{}')",
            primary.pid
        );
        set_error(ErrorType::ValueError, msg.c_str());
        return;
    }
    gun.SetParticleDefinition(definition);
    gun.SetParticleEnergy(primary.energy * CLHEP::MeV);
    gun.SetParticlePosition(G4ThreeVector(
        primary.position[0] * CLHEP::cm,
        primary.position[1] * CLHEP::cm,
        primary.position[2] * CLHEP::cm
    ));
    gun.SetParticleMomentumDirection(G4ThreeVector(
        primary.direction[0],
        primary.direction[1],
        primary.direction[2]
    ));
    gun.GeneratePrimaryVertex(event);
}

SourceImpl * SourceImpl::Get() {
    static SourceImpl * instance = new SourceImpl();
    return instance;
}
