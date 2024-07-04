#include "calzone.h"
#include "sampler.h"
// Geant4 interface.
#include "G4SDManager.hh"

// XXX Document samplers and roles.

SamplerImpl::SamplerImpl(const std::string & name, Roles r) :
    G4VSensitiveDetector(name) {
    this->roles = std::move(r);
}

G4bool SamplerImpl::ProcessHits(G4Step * step, G4TouchableHistory *) {
    if (RUN_AGENT->is_deposits() && this->roles.deposits == Action::Record) {
        double deposit = step->GetTotalEnergyDeposit() / CLHEP::MeV;
        if (deposit == 0.0) {
            return false;
        } else {
            auto && pre = step->GetPreStepPoint();
            auto && post = step->GetPostStepPoint();
            auto && volume = pre->GetPhysicalVolume();
            double non_ionising = step->GetNonIonizingEnergyDeposit() /
                CLHEP::MeV;
            auto start = pre->GetPosition() / CLHEP::cm;
            auto end = post->GetPosition() / CLHEP::cm;

            RUN_AGENT->push_deposit(volume, deposit, non_ionising, start, end);
        }
    }

    if (RUN_AGENT->is_particles() && step->IsLastStepInVolume()) {
        auto && track = step->GetTrack();
        auto && action = this->roles.outgoing;
        if ((action == Action::Catch) ||
            (action == Action::Record)) {
            auto && point = step->GetPostStepPoint();
            auto && volume = point->GetPhysicalVolume();
            auto && pid = track
                ->GetParticleDefinition()
                ->GetPDGEncoding();
            auto && r = point->GetPosition() / CLHEP::cm;
            auto && u = point->GetMomentumDirection();
            Particle particle = {
                pid,
                point->GetKineticEnergy() / CLHEP::MeV,
                { r.x(), r.y(), r.z() },
                { u.x(), u.y(), u.z() },
            };
            RUN_AGENT->push_particle(volume, std::move(particle));
        }
        if ((action == Action::Catch) ||
            (action == Action::Kill)) {
            track->SetTrackStatus(fStopAndKill);
        }
    }

    return true;
}
