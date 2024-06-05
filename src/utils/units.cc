// User interface.
#include "calzone.h"
// Geant4 interface.
#include "G4UnitsTable.hh"


void export_units(rust::Vec<UnitDefinition> & units) {
    auto table = G4UnitDefinition::GetUnitsTable();
    for (auto category: table) {
        for (auto unit: category->GetUnitsList()) {
            UnitDefinition definition = {
                unit->GetName(),
                unit->GetSymbol(),
                unit->GetValue(),
            };
            units.push_back(definition);
        }
    }
}
