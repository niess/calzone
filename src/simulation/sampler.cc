#include "calzone.h"
#include "sampler.h"
// Geant4 interface.
#include "G4SDManager.hh"


SamplerImpl::SamplerImpl() :
    G4VSensitiveDetector("sampler") {
    this->collectionName.insert("deposits");
}

G4bool SamplerImpl::ProcessHits(G4Step * step, G4TouchableHistory *) {
    double deposit = step->GetTotalEnergyDeposit() / CLHEP::MeV;
    if (deposit == 0.0) {
        return false;
    } else {
        auto && pre = step->GetPreStepPoint();
        auto && post = step->GetPostStepPoint();
        auto && volume = pre->GetPhysicalVolume();
        double non_ionising = step->GetNonIonizingEnergyDeposit() / CLHEP::MeV;
        auto start = pre->GetPosition() / CLHEP::cm;
        auto end = post->GetPosition() / CLHEP::cm;

        RUN_AGENT->push_deposit(volume, deposit, non_ionising, start, end);
        return true;
    }
}

SamplerImpl * SamplerImpl::Get() {
    static SamplerImpl * instance = new SamplerImpl();
    G4SDManager::GetSDMpointer()->AddNewDetector(instance);
    return instance;
}
