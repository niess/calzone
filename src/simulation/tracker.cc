#include "calzone.h"
#include "tracker.h"
// Geant4 interface.
#include "G4Step.hh"
#include "G4Track.hh"
#include "G4VProcess.hh"


// ============================================================================
//
// Translation rules.
//
// ============================================================================

struct Translator {
    Translator() {
        this->dictionary["annihil"] = "Annihilation";
        this->dictionary["compt"] = "Compton";
        this->dictionary["conv"] = "Conversion";
        this->dictionary["CoulombScat"] = "Coulomb";
        this->dictionary["eBrem"] = "Bremsstrahlung";
        this->dictionary["eIoni"] = "Ionisation";
        this->dictionary["electronNuclear"] = "Photonuclear";
        this->dictionary["hadElastic"] = "Elastic";
        this->dictionary["hBrems"] = "Bresstrahlung";
        this->dictionary["hIoni"] = "Ionisation";
        this->dictionary["hPairProd"] = "PairProduction";
        this->dictionary["ionIoni"] = "Ionisation";
        this->dictionary["muIoni"] = "Ionisation";
        this->dictionary["muMinusCaptureAtRest"] = "Capture";
        this->dictionary["muonNuclear"] = "Photonuclear";
        this->dictionary["muPairProd"] = "PairProduction";
        this->dictionary["msc"] = "Elastic";
        this->dictionary["nCapture"] = "Capture";
        this->dictionary["neutronInelastic"] = "Inelastic";
        this->dictionary["phot"] = "Photoelectric";
        this->dictionary["protonInelastic"] = "Inelastic";
        this->dictionary["Rayl"] = "Rayleigh";
        this->dictionary["Transportation"] = "Transport";
    }

    const std::string & translate(const std::string & word) {
        try {
            return this->dictionary.at(word);
        } catch (std::out_of_range & e) {
            return word;
        }
    }

    std::map<std::string, std::string> dictionary;
};

static Translator TRANSLATOR;


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
        auto && name = TRANSLATOR.translate(creator->GetProcessName());
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
    auto && track = step->GetTrack();
    auto && tid = track->GetTrackID();
    auto push_vertex = [&](const G4StepPoint * p) {
        auto && r = p->GetPosition() / CLHEP::cm;
        auto && u = p->GetMomentumDirection();
        Vertex vertex = {
            0,
            tid,
            p->GetKineticEnergy() / CLHEP::MeV,
            { r.x(), r.y(), r.z() },
            { u.x(), u.y(), u.z() },
            0x0
        };
        auto process = p->GetProcessDefinedStep();
        if (process != nullptr) {
            auto && name = TRANSLATOR.translate(process->GetProcessName());
            auto dst = (char *)(&vertex.process);
            std::strncpy(
                dst,
                name.c_str(),
                sizeof(vertex.process) - 1
            );
        }
        RUN_AGENT->push_vertex(std::move(vertex));
    };

    if (track->GetCurrentStepNumber() == 1) {
        push_vertex(step->GetPreStepPoint());
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
