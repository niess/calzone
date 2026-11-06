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

void PhysicsImpl::Update(const Physics & definition) {
    bool modified = false;
    if (!this->decayPhysics) {
        this->decayPhysics.reset(new G4DecayPhysics());
        modified = true;
    }

    if (this->current_em_model != definition.em_model) {
        switch (definition.em_model) {
            case EmPhysicsModel::Dna:
                this->emPhysics.reset(new G4EmDNAPhysics());
                break;
            case EmPhysicsModel::Livermore:
                this->emPhysics.reset(new G4EmLivermorePhysics());
                break;
            case EmPhysicsModel::None:
                this->emPhysics.reset(nullptr);
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
        if (definition.em_model == EmPhysicsModel::None) {
            if (this->extraPhysics) {
                this->extraPhysics.reset(nullptr);
            }
        } else {
            if (!this->extraPhysics) {
                this->extraPhysics.reset(new G4EmExtraPhysics());
            }
        }
        this->current_em_model = definition.em_model;
        modified = true;
    }

    if (this->current_had_model != definition.had_model) {
        if (definition.had_model == HadPhysicsModel::None) {
            if (this->hePhysics) {
                this->hePhysics.reset(nullptr);
            }
            if (this->stoppingPhysics) {
                this->stoppingPhysics.reset(nullptr);
            }
            if (this->ionPhysics) {
                this->ionPhysics.reset(nullptr);
            }
        } else {
            if (!this->hePhysics) {
                this->hePhysics.reset(new G4HadronElasticPhysics());
            }
            if (!this->stoppingPhysics) {
                this->stoppingPhysics.reset(new G4StoppingPhysics());
            }
            if (!this->ionPhysics) {
                this->ionPhysics.reset(new G4IonPhysics());
            }
        }
        switch (definition.had_model) {
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
                this->hadPhysics.reset(nullptr);
                break;
        }
        this->current_had_model = definition.had_model;
        modified = true;
    }

    double value = definition.default_cut * CLHEP::cm;
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

double PhysicsCalculator::compute_dedx(double energy) {
    if (this->process.empty()) {
        return this->emCalculator->ComputeTotalDEDX(
            energy * CLHEP::MeV,
            this->particle,
            this->material
        );
    } else {
        return this->emCalculator->ComputeDEDX(
            energy * CLHEP::MeV,
            this->particle,
            this->process,
            this->material
        );
    }
}

// Validate that a material is currently in use by scanning the production cuts
// table. Alternatively, the current geometry could be scanned, but I am
// assuming that this would be less efficient, on average.
static bool has_cuts(const G4Material * material) {
    G4ProductionCutsTable * table =
        G4ProductionCutsTable::GetProductionCutsTable();
    G4int n = table->GetTableSize();
    for (G4int i = 0; i < n; i++) {
        const G4MaterialCutsCouple * couple = table->GetMaterialCutsCouple(i);
        if (couple && couple->GetMaterial() == material) {
            std::cout << "XXX Found " << material->GetName() << " in ProductionCuts table" << std::endl;
            return true; 
        }
    }
    return false;
}

std::unique_ptr<PhysicsCalculator> create_physics_calculator(
    Physics physics,
    int pid,
    rust::Str material,
    rust::Str process
) {
    clear_error();

    G4Material * mat = G4Material::GetMaterial(
        std::string(material), false
    );
    if (mat == nullptr) {
        std::string msg = fmt::format(
            "unknown material '{}'", std::string(material)
        );
        set_error(ErrorType::ValueError, msg.c_str());
        return nullptr;
    }

    if ((GeometryImpl::Get()->State() == GeometryState::Run) && has_cuts(mat)) {
        // Use the current run geometry.
        PhysicsCalculator::DRY_WORLD_MATERIAL = nullptr;
    } else {
        // Trigger a geometry or/and material update.
        PhysicsCalculator::DRY_WORLD_MATERIAL = mat;
    }
    dry_simulation(physics); // Configure / update Geant4 kernel.

    G4ParticleTable * particleTable = G4ParticleTable::GetParticleTable();
    const G4ParticleDefinition * part = particleTable->FindParticle(pid);
    if (part == nullptr) {
        std::string msg = fmt::format("unknown pid '{}'", pid);
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
                "unknown process '{}' for {} (pid={})",
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
