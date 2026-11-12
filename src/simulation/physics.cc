// User interface.
#include "geometry.h"
#include "physics.h"
// fmt library.
#include <fmt/core.h>
// Geant4 interface.
#include "G4EmCalculator.hh"
#include "G4EmDNAPhysics.hh"
#include "G4EmLivermorePhysics.hh"
#include "G4EmStandardPhysics.hh"
#include "G4EmStandardPhysics_option1.hh"
#include "G4EmStandardPhysics_option2.hh"
#include "G4EmStandardPhysics_option3.hh"
#include "G4EmStandardPhysics_option4.hh"
#include "G4EmPenelopePhysics.hh"
#include "G4HadronPhysicsFTFP_BERT.hh"
#include "G4HadronPhysicsFTFP_BERT_HP.hh"
#include "G4HadronPhysicsQGSP_BERT.hh"
#include "G4HadronPhysicsQGSP_BERT_HP.hh"
#include "G4HadronPhysicsQGSP_BIC.hh"
#include "G4HadronPhysicsQGSP_BIC_HP.hh"
#include "G4ProcessTable.hh"
#include "G4RunManager.hh"
#include "G4UImanager.hh"


void PhysicsImpl::ConstructParticle() {
    if (this->decayPhysics) {
        this->decayPhysics->ConstructParticle();
    }
    if (this->emPhysics) {
        this->emPhysics->ConstructParticle();
    }
    if (this->extraPhysics) {
        this->extraPhysics->ConstructParticle();
    }
    if (this->hePhysics) {
        this->hePhysics->ConstructParticle();
    }
    if (this->hadPhysics) {
        this->hadPhysics->ConstructParticle();
    }
    if (this->stoppingPhysics) {
        this->stoppingPhysics->ConstructParticle();
    }
    if (this->ionPhysics) {
        this->ionPhysics->ConstructParticle();
    }
}

void PhysicsImpl::ConstructProcess() {
    this->AddTransportation();
    if (this->decayPhysics) {
        this->decayPhysics->ConstructProcess();
    }
    if (this->emPhysics) {
        this->emPhysics->ConstructProcess();
    }
    if (this->extraPhysics) {
        this->extraPhysics->ConstructProcess();
    }
    if (this->hePhysics) {
        this->hePhysics->ConstructProcess();
    }
    if (this->hadPhysics) {
        this->hadPhysics->ConstructProcess();
    }
    if (this->stoppingPhysics) {
        this->stoppingPhysics->ConstructProcess();
    }
    if (this->ionPhysics) {
        this->ionPhysics->ConstructProcess();
    }
}

void PhysicsImpl::DisableVerbosity() const
{
    auto UImanager = G4UImanager::GetUIpointer();
    UImanager->ApplyCommand("/process/em/verbose 0");
    UImanager->ApplyCommand("/process/had/verbose 0");
}

void PhysicsImpl::Update() {
    static bool initialised = false;
    if (initialised) {
        return;
    } else {
        initialised = true;
        auto && settings = this->settings;
        this->decayPhysics.reset(new G4DecayPhysics());

        if (settings.em_model != EmPhysicsModel::None) {
            switch (settings.em_model) {
                case EmPhysicsModel::Dna:
                    get_material("G4_WATER"); // Required by Geant4/DNA.
                    this->emPhysics.reset(new G4EmDNAPhysics());
                    break;
                case EmPhysicsModel::Livermore:
                    this->emPhysics.reset(new G4EmLivermorePhysics());
                    break;
                case EmPhysicsModel::None:
                    // unreachable.
                    break;
                case EmPhysicsModel::Option1:
                    this->emPhysics.reset(new G4EmStandardPhysics_option1());
                    break;
                case EmPhysicsModel::Option2:
                    this->emPhysics.reset(new G4EmStandardPhysics_option2());
                    break;
                case EmPhysicsModel::Option3:
                    this->emPhysics.reset(new G4EmStandardPhysics_option3());
                    break;
                case EmPhysicsModel::Option4:
                    this->emPhysics.reset(new G4EmStandardPhysics_option4());
                    break;
                case EmPhysicsModel::Penelope:
                    this->emPhysics.reset(new G4EmPenelopePhysics());
                    break;
                case EmPhysicsModel::Standard:
                    this->emPhysics.reset(new G4EmStandardPhysics());
                    break;
            }
            this->extraPhysics.reset(new G4EmExtraPhysics());
        }

        if (settings.had_model != HadPhysicsModel::None) {
            this->hePhysics.reset(new G4HadronElasticPhysics());
            this->stoppingPhysics.reset(new G4StoppingPhysics());
            this->ionPhysics.reset(new G4IonPhysics());

            switch (settings.had_model) {
                case HadPhysicsModel::FTFP_BERT:
                    this->hadPhysics.reset(new G4HadronPhysicsFTFP_BERT());
                    break;
                case HadPhysicsModel::FTFP_BERT_HP:
                    this->hadPhysics.reset(new G4HadronPhysicsFTFP_BERT_HP());
                    break;
                case HadPhysicsModel::QGSP_BERT:
                    this->hadPhysics.reset(new G4HadronPhysicsQGSP_BERT());
                    break;
                case HadPhysicsModel::QGSP_BERT_HP:
                    this->hadPhysics.reset(new G4HadronPhysicsQGSP_BERT_HP());
                    break;
                case HadPhysicsModel::QGSP_BIC:
                    this->hadPhysics.reset(new G4HadronPhysicsQGSP_BIC());
                    break;
                case HadPhysicsModel::QGSP_BIC_HP:
                    this->hadPhysics.reset(new G4HadronPhysicsQGSP_BIC_HP());
                    break;
                case HadPhysicsModel::None:
                    // unreachable.
                    break;
            }
        }
    }

    bool modified = false;
    double value = this->settings.default_cut * CLHEP::cm;
    if (this->GetDefaultCutValue() != value) {
        if (this->isSetDefaultCutValue) {
            this->SetDefaultCutValue(value);
        } else {
            this->defaultCutValue = value;
        }
        modified = true;
    }

    if (modified) {
        auto manager = G4RunManager::GetRunManager();
        if (manager != nullptr) {
            manager->PhysicsHasBeenModified();
        }
    }
}

PhysicsImpl * PhysicsImpl::Get() {
    static PhysicsImpl * instance = new PhysicsImpl();
    return instance;
}

double get_default_cut() {
    return PhysicsImpl::Get()->Settings().default_cut;
}

void set_default_cut(double value) {
    PhysicsImpl::Get()->Settings().default_cut = value;
}

EmPhysicsModel get_em_model() {
    return PhysicsImpl::Get()->Settings().em_model;
}

void set_em_model(EmPhysicsModel model) {
    PhysicsImpl::Get()->Settings().em_model = model;
}

HadPhysicsModel get_had_model() {
    return PhysicsImpl::Get()->Settings().had_model;
}

void set_had_model(HadPhysicsModel model) {
    PhysicsImpl::Get()->Settings().had_model = model;
}

// ============================================================================
//
// Physics calculator interface.
//
// ============================================================================

PhysicsCalculator::PhysicsCalculator(
        G4EmCalculator * calc,
        const G4ParticleDefinition * part,
        const G4Material * mat,
        const std::string proc
): emCalculator(calc), particle(part), material(mat), process(proc)
{}

PhysicsCalculator::~PhysicsCalculator() {
    // Terminate the dry run.
    static G4RunManager * manager = G4RunManager::GetRunManager();
    manager->RunTermination();

    delete this->emCalculator;
}

double PhysicsCalculator::cross_section(
    double energy, bool range_cut, double energy_cut) {
    double cs = 0.0;
    if (range_cut) {
        cs = this->emCalculator->GetCrossSectionPerVolume(
            energy * CLHEP::MeV,
            this->particle,
            this->process,
            this->material
        );
    } else {
        cs = this->emCalculator->ComputeCrossSectionPerVolume(
            energy * CLHEP::MeV,
            this->particle,
            this->process,
            this->material,
            energy_cut * CLHEP::MeV
        );
    }
    return cs * CLHEP::cm;
}

double PhysicsCalculator::energy_cut(bool default_cut, double range_cut) {
    if (default_cut) {
        range_cut = PhysicsImpl::Get()->GetDefaultCutValue();
    } else {
        range_cut *= CLHEP::cm;
    }
    double energy_cut = this->emCalculator->ComputeEnergyCutFromRangeCut(
        range_cut,
        this->particle,
        this->material
    );
    return energy_cut / CLHEP::MeV;
}

bool PhysicsCalculator::production_cuts() const {
    auto index = G4ProductionCuts::GetIndex(this->particle);
    return index >= 0;
}

double PhysicsCalculator::range(double energy, bool range_cut) {
    double r = 0.0;
    if (range_cut) {
        r = this->emCalculator->GetRange(
            energy * CLHEP::MeV,
            this->particle,
            this->material
        );
    } else {
        r = this->emCalculator->GetCSDARange(
            energy * CLHEP::MeV,
            this->particle,
            this->material
        );
    }
    return r / CLHEP::cm;
}

double PhysicsCalculator::stopping_power(
    double energy, bool range_cut, double energy_cut) {
    double s = 0.0;
    if (this->process.empty()) {
        if (range_cut) {
            s = this->emCalculator->GetDEDX(
                energy * CLHEP::MeV,
                this->particle,
                this->material
            );
        } else {
            s = this->emCalculator->ComputeTotalDEDX(
                energy * CLHEP::MeV,
                this->particle,
                this->material
            );
        }
    } else {
        s = this->emCalculator->ComputeDEDX(
            energy * CLHEP::MeV,
            this->particle,
            this->process,
            this->material,
            energy_cut
        );
    }
    return s * CLHEP::cm / CLHEP::MeV;
}

std::unique_ptr<PhysicsCalculator> create_physics_calculator(
    int pid,
    rust::String material,
    rust::Str process
) {
    clear_error();

    G4Material * mat = get_material(material);
    if (mat == nullptr) {
        std::string msg = fmt::format(
            "undefined material '{}'", std::string(material)
        );
        set_error(ErrorType::ValueError, msg.c_str());
        return nullptr;
    }

    PhysicsCalculator::DRY_WORLD_MATERIAL = mat;
    G4EmParameters::Instance()->SetBuildCSDARange(true);
    dry_simulation(); // Configure / update Geant4 kernel.

    G4ParticleTable * particleTable = G4ParticleTable::GetParticleTable();
    const G4ParticleDefinition * part = particleTable->FindParticle(pid);
    if (part == nullptr) {
        std::string msg = fmt::format("undefined pid '{}'", pid);
        set_error(ErrorType::ValueError, msg.c_str());
        return nullptr;
    }

    std::string processName = "";
    if (!process.empty()) {
        G4ProcessTable * processTable = G4ProcessTable::GetProcessTable();
        G4VProcess * proc = processTable->FindProcess(
            std::string(process), part
        );
        if (proc == nullptr) {
            std::string msg = fmt::format(
                "undefined process '{}' for {} (pid={})",
                std::string(process),
                part->GetParticleName(),
                pid
            );
            set_error(ErrorType::ValueError, msg.c_str());
            return nullptr;
        }
        processName = proc->GetProcessName();
    }

    G4EmCalculator * calc = new G4EmCalculator;
    if (calc == nullptr) {
        set_error(
            ErrorType::Geant4Exception,
            "could not create G4EmCalculator"
        );
        return nullptr;
    }

    return std::make_unique<PhysicsCalculator>(calc, part, mat, processName);
}
