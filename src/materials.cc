#include "calzone.h"
// Standard library.
#include <map>
// fmt library.
#include <fmt/core.h>
// Geant4 interface.
#include "G4Element.hh"
#include "G4NistManager.hh"
#include "G4SystemOfUnits.hh"


// ============================================================================
//
// Atomic elements interface.
//
// ============================================================================

static std::map<rust::String, G4Element *> ELEMENTS;

static G4Element * get_element(const rust::String & name, bool ignore=true) {
    G4Element * element = nullptr;

    auto nist = G4NistManager::Instance();
    element = nist->FindOrBuildElement(std::string(name));
    if (element != nullptr) {
        return element;
    }

    try {
        element = ELEMENTS.at(name);
    } catch (std::out_of_range & _) {
        if (!ignore) {
            auto msg = fmt::format(
                "bad element (undefined {})", std::string(name)
            );
            set_error(ErrorType::ValueError, msg.c_str());
        }
    }
    return element;
}

std::shared_ptr<Error> add_element(const Element & e) {
    clear_error();

    G4Element * element = get_element(e.name);
    if (element != nullptr) {
        if ((element->GetSymbol() != std::string(e.symbol)) ||
            (element->GetZ() != e.Z) ||
            (element->GetAtomicMassAmu() != e.A)) {
            auto msg = fmt::format(
                "bad element (redefinition of '{}')", 
                std::string(e.name)
            );
            set_error(ErrorType::ValueError, msg.c_str());
        }
        return get_error();
    }

    element = new G4Element(
        std::string(e.name),
        std::string(e.symbol),
        e.Z,
        e.A * (CLHEP::g / CLHEP::mole)
    );
    if (element == nullptr) {
        if (!any_error()) {
            auto msg = fmt::format(
                "bad element (could not create '{}')",
                std::string(e.name)
            );
            set_error(ErrorType::ValueError, msg.c_str());
        }
        return get_error();
    }
    ELEMENTS[e.name] = element;

    return get_error();
}


// ============================================================================
//
// Materials interface.
//
// ============================================================================

struct HashedMaterial {
    G4Material * material;
    uint64_t hash;
};

static std::map<rust::String, HashedMaterial> MATERIALS;

static HashedMaterial get_material(
    const rust::String & name,
    bool ignore=true
) {
    HashedMaterial hashed = { nullptr, 0x0 };

    auto nist = G4NistManager::Instance();
    hashed.material = nist->FindOrBuildMaterial(std::string(name));
    if (hashed.material != nullptr) {
        return hashed;
    }

    try {
        hashed = MATERIALS.at(name);
    } catch (std::out_of_range & _) {
        if (!ignore) {
            auto msg = fmt::format(
                "bad material (undefined '{}')",
                std::string(name)
            );
            set_error(ErrorType::ValueError, msg.c_str());
        }
    }
    return hashed;
}

static G4Material * create_material(
    const MaterialProperties & properties,
    int n
) {
    clear_error();

    const double density = std::max(
        properties.density * (CLHEP::g / CLHEP::cm3),
        CLHEP::universe_mean_density
    );
    auto material = new G4Material(
        std::string(properties.name),
        density,
        n,
        properties.state
    );
    if (material == nullptr) {
        if (!any_error()) {
            auto msg = fmt::format(
                "bad material (could not create '{}')",
                std::string(properties.name)
            );
            set_error(ErrorType::ValueError, msg.c_str());
        }
    }
    return material;
}

std::shared_ptr<Error> add_mixture(const Mixture & mixture) {
    auto hash = mixture.get_hash();
    HashedMaterial hashed = get_material(mixture.properties.name);
    if (hashed.material != nullptr) {
        if (hashed.hash != hash) {
            auto msg = fmt::format(
                "bad material (redefinition of '{}')",
                std::string(mixture.properties.name)
            );
            set_error(ErrorType::ValueError, msg.c_str());
        }
        return get_error();
    }

    G4Material * material = create_material(
        mixture.properties,
        mixture.components.size()
    );
    if (material == nullptr) {
        return get_error();
    }
    for (auto component: mixture.components) {
        auto element = get_element(component.name);
        if (element != nullptr) {
            material->AddElement(element, component.weight);
        } else {
            HashedMaterial hashed = get_material(component.name);
            if (hashed.material == nullptr) {
                delete material;
                auto msg = fmt::format(
                    "bad component for '{}' material (undefined '{}')",
                    std::string(mixture.properties.name),
                    std::string(component.name)
                );
                set_error(ErrorType::ValueError, msg.c_str());
                return get_error();
            }
            material->AddMaterial(hashed.material, component.weight);
        }
    }

    MATERIALS[mixture.properties.name] = { material, hash };

    return get_error();
}

std::shared_ptr<Error> add_molecule(const Molecule & molecule) {
    auto hash = molecule.get_hash();
    HashedMaterial hashed = get_material(molecule.properties.name);
    if (hashed.material != nullptr) {
        if (hashed.hash != hash) {
            auto msg = fmt::format(
                "bad material (redefinition of '{}')",
                std::string(molecule.properties.name)
            );
            set_error(ErrorType::ValueError, msg.c_str());
        }
        return get_error();
    }

    G4Material * material = create_material(
        molecule.properties,
        molecule.components.size()
    );
    if (material == nullptr) {
        return get_error();
    }
    for (auto component: molecule.components) {
        auto element = get_element(component.name);
        if (element == nullptr) {
            delete material;
            auto msg = fmt::format(
                "bad component for '{}' material (undefined '{}' element)",
                std::string(molecule.properties.name),
                std::string(component.name)
            );
            set_error(ErrorType::ValueError, msg.c_str());
            return get_error();
        }
        material->AddElement(element, (int)component.weight);
    }

    MATERIALS[molecule.properties.name] = { material, hash };

    return get_error();
}
