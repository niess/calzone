// User interface.
#include "calzone.h"
#include "simulation/geometry.h"
#include "simulation/physics.h"
#include "simulation/random.h"
#include "simulation/sampler.h"
#include "simulation/source.h"
#include "simulation/tracker.h"
// Geant4 interface.
#include "G4RunManager.hh"
#include "G4UImanager.hh"
#include "Randomize.hh"


RunAgent * RUN_AGENT = nullptr;


void drop_simulation() {
    // Gracefully exit Geant4. That is, detach any pending geometry before
    // deleting the run manager.
    GeometryImpl::Get()->Reset();
    auto manager = G4RunManager::GetRunManager();
    delete manager;
}

std::shared_ptr<Error> run_simulation(
    RunAgent & agent,
    RandomContext &, // Implicit scope.
    bool verbose
) {
    clear_error();

    // Configure the simulation.
    auto geometryImpl = GeometryImpl::Get();
    auto physicsImpl = PhysicsImpl::Get();

    RUN_AGENT = &agent;
    geometryImpl->Update();
    physicsImpl->Update();

    physicsImpl->DisableVerbosity();
    static G4RunManager * manager = nullptr;
    if (manager == nullptr) {
        auto buffer = std::cout.rdbuf();
        std::cout.rdbuf(nullptr); // Disable cout temporarly.
        manager = new G4RunManager();
        std::cout.rdbuf(buffer);

        manager->SetUserInitialization(physicsImpl);
        manager->SetUserInitialization(geometryImpl);

        auto sourceImpl = SourceImpl::Get(); // Must be after geometry and
                                             // physics.
        manager->SetUserAction(sourceImpl);
        manager->SetUserAction(SteppingImpl::Get());
        manager->Initialize();
    }

    if (RUN_AGENT->is_tracker()) {
        manager->SetUserAction(TrackingImpl::Get());
    } else {
        manager->SetUserAction(TrackingImpl::None());
    }

    if (RUN_AGENT->is_secondaries()) {
        manager->SetUserAction(StackingImpl::None());
    } else {
        manager->SetUserAction(StackingImpl::Get());
    }

    if (verbose) {
        auto ui = G4UImanager::GetUIpointer();
        ui->ApplyCommand("/tracking/verbose 1");
    }

    // Process events in bunches (in order to check for Ctrl+C).
    constexpr int bunch_size = 100;
    const size_t n = agent.events();
    const size_t a = n / bunch_size;
    const int b = n % bunch_size;
    for (size_t i = 0; i <= a; i++) {
        int r = (i < a) ? bunch_size : b;
        if (r > 0) {
            manager->BeamOn(r);
        }
        if (any_error()) break;
    }

    return get_error();
}
