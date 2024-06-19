#include "calzone.h"
// Geant4 interface.
#include "G4String.hh"


rust::Str as_str(const G4String & value) {
    return rust::Str(value);
}

std::array<double, 3> to_vec(const G4ThreeVector & value) {
    return { value.x(), value.y(), value.z() };
}
