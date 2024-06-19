#include "calzone.h"
#include "sampler.h"
// Geant4 interface.
#include "G4SDManager.hh"


SamplerImpl::SamplerImpl() :
    G4VSensitiveDetector("sampler") {
    this->collectionName.insert("hits");

    // Translation.
    this->processes["annihil"] = "Annihilation";
    this->processes["compt"] = "Compton";
    this->processes["conv"] = "Conversion";
    this->processes["CoulombScat"] = "Coulomb";
    this->processes["eBrem"] = "Bremsstrahlung";
    this->processes["eIoni"] = "Ionisation";
    this->processes["electronNuclear"] = "Photonuclear";
    this->processes["hadElastic"] = "Elastic";
    this->processes["hBrems"] = "Bresstrahlung";
    this->processes["hIoni"] = "Ionisation";
    this->processes["hPairProd"] = "PairProduction";
    this->processes["ionIoni"] = "Ionisation";
    this->processes["muIoni"] = "Ionisation";
    this->processes["muMinusCaptureAtRest"] = "Capture";
    this->processes["muonNuclear"] = "Photonuclear";
    this->processes["muPairProd"] = "PairProduction";
    this->processes["msc"] = "Elastic";
    this->processes["nCapture"] = "Capture";
    this->processes["neutronInelastic"] = "Inelastic";
    this->processes["phot"] = "Photoelectric";
    this->processes["protonInelastic"] = "Inelastic";
    this->processes["Rayl"] = "Rayleigh";
    this->processes["Transportation"] = "Transport";
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
