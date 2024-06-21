#include "G4UserSteppingAction.hh"
#include "G4UserTrackingAction.hh"

class G4Step;
class G4Track;

struct TrackingImpl : public G4UserTrackingAction {
    TrackingImpl(const TrackingImpl &) = delete;

    // Geant4 interface.
    void PreUserTrackingAction(const G4Track *);

    // User interface.
    static TrackingImpl * Get();
    static TrackingImpl * None();

private:
    TrackingImpl() = default;
};

struct SteppingImpl : public G4UserSteppingAction {
    SteppingImpl(const SteppingImpl &) = delete;

    // Geant4 interface.
    void UserSteppingAction (const G4Step *);

    // User interface.
    static SteppingImpl * Get();
    static SteppingImpl * None();

private:
    SteppingImpl() = default;
};
