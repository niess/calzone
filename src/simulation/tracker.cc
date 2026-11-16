#include "calzone.h"
#include "physics.h"
#include "sampler.h"
#include "tracker.h"
// Geant4 interface.
#include "G4Step.hh"
#include "G4Track.hh"
#include "G4VProcess.hh"


// ============================================================================
//
// Stacking implementation.
//
// ============================================================================

G4ClassificationOfNewTrack StackingImpl::ClassifyNewTrack (
    const G4Track * track
) {
    if (track->GetParentID() == 0) {
        return fUrgent;
    } else {
        return fKill;
    }
}

StackingImpl * StackingImpl::Get() {
    static StackingImpl * instance = new StackingImpl();
    return instance;
}

StackingImpl * StackingImpl::None() {
    return nullptr;
}


// ============================================================================
//
// Tracking implementation.
//
// ============================================================================

void TrackingImpl::PreUserTrackingAction(const G4Track * track) {
    auto && parent = track->GetParentID();
    struct Track data = {
        0,
        track->GetTrackID(),
        parent,
        track->GetParticleDefinition()->GetPDGEncoding(),
        0x0
    };
    auto creator = track->GetCreatorProcess();
    if (creator != nullptr) {
        auto && name = ProcessTranslator::Get()->Translate(
            creator->GetProcessName()
        );
        auto dst = (char *)(&data.creator);
        std::strncpy(
            dst,
            name.c_str(),
            sizeof(data.creator) - 1
        );
    } else if (parent == 0) {
        auto dst = (char *)(&data.creator);
        std::strncpy(
            dst,
            "Primary",
            sizeof(data.creator) - 1
        );
    }
    RUN_AGENT->push_track(std::move(data));
}

TrackingImpl * TrackingImpl::Get() {
    static TrackingImpl * instance = new TrackingImpl();
    return instance;
}

TrackingImpl * TrackingImpl::None() {
    return nullptr;
}


// ============================================================================
//
// Stepping implementation.
//
// ============================================================================

void SteppingImpl::UserSteppingAction(const G4Step * step) {
    if (RUN_AGENT->is_particles() && step->IsLastStepInVolume()) {
        auto && point = step->GetPostStepPoint();
        G4VPhysicalVolume * volume = point->GetPhysicalVolume();
        if (volume != nullptr) {
            auto && sensitive = static_cast<SamplerImpl *>(
                volume->GetLogicalVolume()->GetSensitiveDetector()
            );
            if (sensitive != nullptr) {
                auto && track = step->GetTrack();
                auto && tid = track->GetTrackID();
                auto && action = sensitive->roles.ingoing;
                if ((action == Action::Catch) ||
                    (action == Action::Record)) {
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
                    RUN_AGENT->push_particle(volume, tid, std::move(particle));
                }
                if ((action == Action::Catch) ||
                    (action == Action::Kill)) {
                    track->SetTrackStatus(fStopAndKill);
                }
            }
        }
    }

    if (!RUN_AGENT->is_tracker()) {
        return;
    }

    auto && track = step->GetTrack();
    auto && tid = track->GetTrackID();
    auto push_vertex = [&](const G4StepPoint * p, bool pre=false) {
        auto && r = p->GetPosition() / CLHEP::cm;
        auto && u = p->GetMomentumDirection();
        auto && t = p->GetGlobalTime() / CLHEP::s;
        Vertex vertex = {
            0,
            tid,
            p->GetKineticEnergy() / CLHEP::MeV,
            { r.x(), r.y(), r.z() },
            { u.x(), u.y(), u.z() },
            t,
            { 0x0 },
            { 0x0 }
        };
        auto volume = p->GetPhysicalVolume();
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
        auto process = p->GetProcessDefinedStep();
        if (process != nullptr) {
            auto && name = ProcessTranslator::Get()->Translate(
                process->GetProcessName()
            );
            auto dst = (char *)(&vertex.process);
            std::strncpy(
                dst,
                name.c_str(),
                sizeof(vertex.process) - 1
            );
        } else if ((track->GetTrackStatus() == fStopAndKill) && !pre) {
            auto dst = (char *)(&vertex.process);
            std::strncpy(
                dst,
                "Kill",
                sizeof(vertex.process) - 1
            );
        }
        RUN_AGENT->push_vertex(std::move(vertex));
    };

    if (track->GetCurrentStepNumber() == 1) {
        push_vertex(step->GetPreStepPoint(), true);
    }
    push_vertex(step->GetPostStepPoint());
}

SteppingImpl * SteppingImpl::Get() {
    static SteppingImpl * instance = new SteppingImpl();
    return instance;
}

SteppingImpl * SteppingImpl::None() {
    return nullptr;
}
