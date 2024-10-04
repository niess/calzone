use crate::utils::error::ErrorKind::ValueError;
use crate::utils::error::{Error, variant_error};
use crate::utils::extract::{Extractor, Property, Tag, TryFromBound};
use crate::utils::io::DictLike;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use super::ffi;
use super::volume::Volume;
use std::collections::{HashMap, HashSet};

pub mod gate;
mod hash;

/// Define Geant4 material(s).
#[pyfunction]
pub fn define(definition: DictLike) -> PyResult<()> {
    let tag = Tag::new("", "materials", None);
    let materials = MaterialsDefinition::try_from_dict(&tag, &definition)?;
    materials.build()?;
    Ok(())
}


// ===============================================================================================
//
// Geometry definition.
//
// This is a thin wrapper collecting the top volume description and some optional material
// definitions.
//
// ===============================================================================================

#[derive(Deserialize, Serialize)]
pub struct MaterialsDefinition {
    elements: Vec<ffi::Element>,
    molecules: Vec<ffi::Molecule>,
    mixtures: Vec<ffi::Mixture>,
}

impl MaterialsDefinition {
    pub fn build(&self) -> PyResult<()> {
        for element in &self.elements {
            ffi::add_element(&element)
                .to_result()?;
        }
        for molecule in &self.molecules {
            ffi::add_molecule(&molecule)
                .to_result()?;
        }
        let mixtures = self.sorted_mixtures()?;
        for mixture in mixtures {
            ffi::add_mixture(mixture)
                .to_result()?;
        }
        Ok(())
    }

    pub fn drain(mut slf: Option<Self>, volume: &mut Volume) -> Option<Self> {
        if let Some(materials) = volume.materials.take() {
            match slf.as_mut() {
                None => slf = Some(materials),
                Some(m) => m.extend(materials),
            }
        }
        for daughter in volume.volumes.iter_mut() {
            slf = Self::drain(slf, daughter);
        }
        slf
    }

    pub fn extend(&mut self, mut other: Self) {
        for e in other.elements.drain(..) {
            self.elements.push(e);
        }
        for m in other.molecules.drain(..) {
            self.molecules.push(m);
        }
        for m in other.mixtures.drain(..) {
            self.mixtures.push(m);
        }
    }

    fn sorted_mixtures<'a>(&'a self) -> PyResult<Vec<&'a ffi::Mixture>> {
        if self.mixtures.len() <= 1 {
            let mixtures: Vec<_> = self.mixtures.iter().collect();
            return Ok(mixtures)
        }

        let map: HashMap<&str, &ffi::Mixture> = self.mixtures.iter()
            .map(|mixture| (mixture.properties.name.as_str(), mixture))
            .collect();

        // Find dependencies and look for cycles.
        type Dependencies<'a> = HashSet<&'a str>;

        fn find_deps<'a>(
            root: &str,
            mixture: &ffi::Mixture,
            map: &'a HashMap<&str, &ffi::Mixture>,
            mut deps: Dependencies<'a>,
        ) -> PyResult<Dependencies<'a>> {
            for component in mixture.components.iter() {
                if &component.name == root {
                    let why = format!(
                        "cycle between '{}' and '{}'",
                        root,
                        mixture.properties.name
                    );
                    let err = Error::new(ValueError)
                        .what("mixture")
                        .why(&why);
                    return Err(err.into())
                } else {
                    if let Some((name, mixture)) = map.get_key_value(component.name.as_str()) {
                        deps = find_deps(root, mixture, map, deps)?;
                        deps.insert(name);
                    }
                }
            }
            Ok(deps)
        }

        let mut deps: HashMap<&str, Dependencies> = HashMap::new();
        for mixture in &self.mixtures {
            let name = mixture.properties.name.as_str();
            let mut dep = Dependencies::new();
            dep = find_deps(name, mixture, &map, dep)?;
            deps.insert(name, dep);
        }

        // Sort mixtures.
        let mut mixtures: Vec<_> = self.mixtures.iter().collect();
        let n = mixtures.len();
        let mut i = 0;
        loop {
            let mut j = i;
            let deps = &deps[mixtures[i].properties.name.as_str()];
            for k in (i + 1)..n {
                if deps.contains(mixtures[k].properties.name.as_str()) {
                    j = k
                }
            }
            if j > i {
                mixtures.insert(j + 1, mixtures[i]);
                mixtures.remove(i);
            } else {
                i += 1;
                if i == n - 1 {
                    break;
                }
            }
        }

        Ok(mixtures)
    }
}

impl TryFromBound for MaterialsDefinition {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let py = value.py();
        let tag = tag.cast("materials");
        let materials: DictLike = value
            .extract()
            .map_err(|err|
                tag.bad().why(format!("{}", err.value_bound(py))).to_err(ValueError)
            )?;

        const EXTRACTOR: Extractor<3> = Extractor::new([
            Property::optional_dict("elements"),
            Property::optional_dict("molecules"),
            Property::optional_dict("mixtures"),
        ]);
        let [elements, molecules, mixtures] = EXTRACTOR.extract(&tag, &materials, None)?;

        let elements: Option<DictLike> = elements.into();
        let elements = match elements {
            None => Vec::new(),
            Some(elements) => {
                let tag = tag.cast("element");
                Vec::<ffi::Element>::try_from_dict(&tag, &elements)?
            },
        };

        let molecules: Option<DictLike> = molecules.into();
        let molecules = match molecules {
            None => Vec::new(),
            Some(molecules) => {
                let tag = tag.cast("molecules");
                Vec::<ffi::Molecule>::try_from_dict(&tag, &molecules)?
            },
        };

        let mixtures: Option<DictLike> = mixtures.into();
        let mixtures = match mixtures {
            None => Vec::new(),
            Some(mixtures) => {
                let tag = tag.cast("mixtures");
                Vec::<ffi::Mixture>::try_from_dict(&tag, &mixtures)?
            },
        };

        let materials = Self { elements, molecules, mixtures };
        Ok(materials)
    }

}

// ===============================================================================================
//
// Conversions (from a Python dict).
//
// ===============================================================================================

impl TryFromBound for ffi::Element {
    #[allow(non_snake_case)]
    fn try_from_dict<'py>(tag: &Tag, value: &DictLike<'py>) -> PyResult<Self> {
        const EXTRACTOR: Extractor<3> = Extractor::new([
            Property::required_f64("Z"),
            Property::required_f64("A"),
            Property::optional_str("symbol"),
        ]);

        let tag = tag.cast("element");
        let [Z, A, symbol] = EXTRACTOR.extract(&tag, value, None)?;
        let symbol = if symbol.is_none() { tag.name().to_string() } else { symbol.into() };

        let element = Self {
            name: tag.name().to_string(),
            symbol,
            Z: Z.into(),
            A: A.into(),
        };
        Ok(element)
    }
}

impl TryFromBound for ffi::Molecule {
    fn try_from_dict<'py>(tag: &Tag, value: &DictLike<'py>) -> PyResult<Self> {
        const EXTRACTOR: Extractor<3> = Extractor::new([
            Property::required_f64("density"),
            Property::required_dict("composition"), // XXX Parse from name & document.
            Property::optional_str("state"),
        ]);

        let tag = tag.cast("molecule");
        let (properties, composition) = try_into_properties(&EXTRACTOR, &tag, value)?;
        let components = Vec::<ffi::MoleculeComponent>::try_from_dict(&tag, &composition)?;

        let molecule = Self::new(properties, components);
        Ok(molecule)
    }
}

impl TryFromBound for ffi::Mixture {
    fn try_from_dict<'py>(tag: &Tag, value: &DictLike<'py>) -> PyResult<Self> {
        const EXTRACTOR: Extractor<3> = Extractor::new([
            Property::required_f64("density"),
            Property::required_dict("composition"),
            Property::optional_str("state"),
        ]);

        let tag = tag.cast("mixture");
        let (properties, composition) = try_into_properties(&EXTRACTOR, &tag, value)?;
        let components = Vec::<ffi::MixtureComponent>::try_from_dict(
            &tag, &composition
        )?;

        let mixture = Self::new(properties, components);
        Ok(mixture)
    }
}

fn try_into_properties<'py>(
    extractor: &Extractor<3>,
    tag: &Tag,
    value: &DictLike<'py>
) -> PyResult<(ffi::MaterialProperties, DictLike<'py>)> {
    let [density, composition, state] = extractor.extract(tag, value, None)?;

    let state: ffi::G4State = if state.is_none() {
        ffi::G4State::kStateUndefined
    } else {
        let state: String = state.into();
        let state = State::from_str(state.as_str())
            .map_err(|options| {
                let message: String = tag.bad().what("state").into();
                variant_error(message.as_str(), state.as_str(), options)
            })?;
        state.into()
    };
    let properties = ffi::MaterialProperties {
        name: tag.name().to_string(),
        density: density.into(),
        state,
    };

    let composition: DictLike = composition.into();
    Ok((properties, composition))
}

#[derive(EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="lower_case")]
enum State {
  Gas,
  Liquid,
  Solid,
}

impl From<State> for ffi::G4State {
    fn from(value: State) -> Self {
        match value {
            State::Gas => ffi::G4State::kStateGas,
            State::Liquid => ffi::G4State::kStateLiquid,
            State::Solid => ffi::G4State::kStateSolid,
        }
    }
}

impl TryFromBound for ffi::MoleculeComponent {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let property = Property::required_u32("weight");
        let tag = tag.cast("component");
        let weight = property.extract(&tag, value)?;

        let component = Self {
            name: tag.name().to_string(),
            weight: weight.into(),
        };
        Ok(component)
    }
}

impl TryFromBound for ffi::MixtureComponent {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let property = Property::required_f64("weight");
        let tag = tag.cast("component");
        let weight = property.extract(&tag, value)?;

        let component = Self {
            name: tag.name().to_string(),
            weight: weight.into(),
        };
        Ok(component)
    }
}


// ===============================================================================================
//
// Constructors (ensuring the ordering of components).
//
// ===============================================================================================

impl ffi::Mixture {
    pub fn new(
        properties: ffi::MaterialProperties,
        mut components: Vec<ffi::MixtureComponent>
    ) -> Self {
        components.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Self { properties, components }
    }
}

impl ffi::Molecule {
    pub fn new(
        properties: ffi::MaterialProperties,
        mut components: Vec<ffi::MoleculeComponent>
    ) -> Self {
        components.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Self { properties, components }
    }
}
