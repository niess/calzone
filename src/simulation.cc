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
#include "G4StateManager.hh"
#include "G4UImanager.hh"
#include "Randomize.hh"


RunAgent * RUN_AGENT = nullptr;

void drop_simulation() {
    // Gracefully exit Geant4. That is, detach any pending geometry before
    // deleting the run manager.
    GeometryImpl::Get()->Reset(nullptr);
    auto manager = G4RunManager::GetRunManager();
    delete manager;
}

static G4RunManager * init_manager() {
    auto physicsImpl = PhysicsImpl::Get();
    auto geometryImpl = GeometryImpl::Get();

    geometryImpl->Update();
    physicsImpl->Update();

    static G4RunManager * manager = G4RunManager::GetRunManager();
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

    return manager;
}

void dry_simulation() {
    RUN_AGENT = nullptr;
    G4RunManager * manager = init_manager();
    auto buffer = std::cout.rdbuf();
    std::cout.rdbuf(nullptr); // Disable cout temporarly.
    manager->RunInitialization();
    std::cout.rdbuf(buffer);
}

std::shared_ptr<Error> run_simulation(
    RunAgent & agent,
    RandomContext &, // Implicit scope.
    bool verbose
) {
    clear_error();

    // Configure the simulation.
    RUN_AGENT = &agent;
    G4RunManager * manager = init_manager();

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

    // Initialize the run.
    if (!manager->ConfirmBeamOnCondition()) {
        return get_error();
    }

    manager->ConstructScoringWorlds();

    const std::uint64_t n = agent.events();
    manager->SetNumberOfEventsToBeProcessed(n);
    auto buffer = std::cout.rdbuf();
    std::cout.rdbuf(nullptr); // Disable cout temporarly.
    manager->RunInitialization();
    std::cout.rdbuf(buffer);
    manager->InitializeEventLoop(n);

    // Process events in bunches (in order to check for Ctrl+C).
    constexpr int bunch_size = 100;
    const std::uint64_t a = n / bunch_size;
    const int b = n % bunch_size;
    std::uint64_t i_event = 0;
    for (std::uint64_t i = 0; i <= a; i++) {
        int r = (i < a) ? bunch_size : b;
        if (r > 0) {
            for (int j  = 0; j < r; j++) {
                manager->ProcessOneEvent(i_event++);
                manager->TerminateOneEvent();
            }
        }
        if (any_error()) break;
    }

    manager->TerminateEventLoop();
    manager->RunTermination();

    return get_error();
}

G4ApplicationState simulation_state() {
    return G4StateManager::GetStateManager()->GetCurrentState();
}
