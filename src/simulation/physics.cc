// User interface.
#include "physics.h"
// Geant4 interface.
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
    auto && definition = RUN_AGENT->physics();
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
