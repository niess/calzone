#include "calzone.h"
#include "sampler.h"
// Geant4 interface.
#include "G4SDManager.hh"


SamplerImpl::SamplerImpl(const std::string & name, Roles r) :
    G4VSensitiveDetector(name) {
    this->roles = std::move(r);
}

G4bool SamplerImpl::ProcessHits(G4Step * step, G4TouchableHistory *) {
    if (RUN_AGENT->is_sampler() && this->roles.deposits == Action::Record) {
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

    if (step->IsLastStepInVolume()) {
        // XXX implement catch & record.
        auto && action = this->roles.outgoing;
        if ((action == Action::Catch) ||
            (action == Action::Kill)) {
            step->GetTrack()->SetTrackStatus(fStopAndKill);
        }
    }

    return true;
}
