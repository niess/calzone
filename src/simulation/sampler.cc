#include "calzone.h"
#include "sampler.h"
// Geant4 interface.
#include "G4SDManager.hh"

SamplerImpl::SamplerImpl(const std::string & name, Roles r) :
    G4VSensitiveDetector(name) {
    this->roles = std::move(r);
}

G4bool SamplerImpl::ProcessHits(G4Step * step, G4TouchableHistory *) {
    if (RUN_AGENT->is_deposits() && this->roles.deposits == Action::Record) {
        double deposit = step->GetTotalEnergyDeposit() / CLHEP::MeV;
        if (deposit > 0.0) {
            auto && pre = step->GetPreStepPoint();
            auto && post = step->GetPostStepPoint();
            auto && volume = pre->GetPhysicalVolume();
            auto && track = step->GetTrack();
            double point_deposit = 0.0;
            auto particle = track->GetParticleDefinition();
            if (particle->GetPDGCharge() == 0.0) {
                point_deposit = deposit;
            }
            int tid = track->GetTrackID();
            int pid = particle->GetPDGEncoding();
            double energy = pre->GetKineticEnergy();
            auto start = pre->GetPosition() / CLHEP::cm;
            auto end = post->GetPosition() / CLHEP::cm;

            RUN_AGENT->push_deposit(
                volume, tid, pid, energy, deposit, point_deposit, start, end
            );
        }
    }

    bool outgoing = RUN_AGENT->is_particles() && step->IsLastStepInVolume();
    if (outgoing) {
        // Check that the next volume is not a daughter of the current one.
        auto pre = step->GetPreStepPoint()->GetPhysicalVolume();
        auto post = step->GetPostStepPoint()->GetPhysicalVolume();
        if ((pre != nullptr) && (post != nullptr)) {
            if (post->GetMotherLogical() == pre->GetLogicalVolume()) {
                outgoing = false;
            }
        }
    }
    if (outgoing) {
        auto && track = step->GetTrack();
        auto && action = this->roles.outgoing;
        if ((action == Action::Catch) ||
            (action == Action::Record)) {
            auto && volume = step->GetPreStepPoint()->GetPhysicalVolume();
            auto && tid = track->GetTrackID();
            auto && pid = track
                ->GetParticleDefinition()
                ->GetPDGEncoding();
            auto && point = step->GetPostStepPoint();
            auto && r = point->GetPosition() / CLHEP::cm;
            auto && u = point->GetMomentumDirection();
            Particle particle = {
                pid,
                point->GetKineticEnergy() / CLHEP::MeV,
                { r.x(), r.y(), r.z() },
                { u.x(), u.y(), u.z() },
            };
            RUN_AGENT->push_particle(volume, tid, std::move(particle));
        }
        if ((action == Action::Catch) ||
            (action == Action::Kill)) {
            track->SetTrackStatus(fStopAndKill);

            if (RUN_AGENT->is_tracker()) {
                // Record the last vertex (since killed tracks do not seem to
                // trigger the user stepping action).
                auto && tid = track->GetTrackID();
                auto && point = step->GetPostStepPoint();
                auto && r = point->GetPosition() / CLHEP::cm;
                auto && u = point->GetMomentumDirection();
                Vertex vertex = {
                    0,
                    tid,
                    point->GetKineticEnergy() / CLHEP::MeV,
                    { r.x(), r.y(), r.z() },
                    { u.x(), u.y(), u.z() },
                    { 0x0 },
                    { 0x0 }
                };
                auto volume = point->GetPhysicalVolume();
                if (volume != nullptr) {
                    std::string name;
                    std::istringstream stream(volume->GetName());
                    while (getline(stream, name, '.')) {}
                    auto dst = (char *)(&vertex.volume);
                    std::strncpy(
                        dst,
                        name.c_str(),
                        sizeof(vertex.volume) - 1
                    );
                }
                auto dst = (char *)(&vertex.process);
                std::strncpy(
                    dst,
                    "Kill",
                    sizeof(vertex.process) - 1
                );
                RUN_AGENT->push_vertex(std::move(vertex));
            }
        }
    }

    return true;
}
