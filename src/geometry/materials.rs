use crate::utils::error::ErrorKind::ValueError;
use crate::utils::error::{Error, variant_error};
use crate::utils::extract::{Extractor, Property, Tag, TryFromBound};
use crate::utils::io::DictLike;
use crate::utils::namespace::Namespace;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use super::ffi;
use super::volume::Volume;
use std::collections::{HashMap, HashSet};

pub mod gate;
mod hash;


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
            Property::optional_dict("composition"),
            Property::optional_str("state"),
        ]);

        let tag = tag.cast("molecule");
        let (properties, composition) = try_into_properties(&EXTRACTOR, &tag, value)?;
        let components = match composition {
            Some(composition) => Vec::<ffi::MoleculeComponent>::try_from_dict(&tag, &composition)?,
            None => ffi::MoleculeComponent::try_from_tag(&tag)?
        };

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
            &tag, &composition.unwrap()
        )?;

        let mixture = Self::new(properties, components);
        Ok(mixture)
    }
}

fn try_into_properties<'py>(
    extractor: &Extractor<3>,
    tag: &Tag,
    value: &DictLike<'py>
) -> PyResult<(ffi::MaterialProperties, Option<DictLike<'py>>)> {
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

    let composition: Option<DictLike> = composition.into();
    Ok((properties, composition))
}

#[derive(EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="lower_case")]
pub enum State {
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

impl TryFrom<ffi::G4State> for State {
    type Error = &'static str;

    fn try_from(value: ffi::G4State) -> Result<Self, Self::Error> {
        match value {
            ffi::G4State::kStateGas => Ok(Self::Gas),
            ffi::G4State::kStateLiquid => Ok(Self::Liquid),
            ffi::G4State::kStateSolid => Ok(Self::Solid),
            _ => Err("undefined"),
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

impl ffi::MoleculeComponent {
    fn try_from_tag<'py>(tag: &Tag) -> PyResult<Vec<Self>> {
        let re = Regex::new(r"([A-Z][a-z]?)([0-9]*)").unwrap();
        let mut composition = Vec::<Self>::new();
        for captures in re.captures_iter(tag.name()) {
            let name = captures.get(1).unwrap().as_str().to_string();
            let weight = captures.get(2).unwrap().as_str();
            let weight: u32 = if weight.len() == 0 {
                1
            } else {
                weight.parse::<u32>()
                    .map_err(|err| {
                        let tag = tag.cast("weight");
                        tag.bad().why(format!("{}", err)).to_err(ValueError)
                    })?
            };
            composition.push(Self { name, weight });
        }
        if composition.is_empty() {
            let tag = tag.cast("weight");
            let err = tag.bad().why("bad chemical formula, or missing 'composition'".to_owned())
                .to_err(ValueError);
            Err(err)
        } else {
            Ok(composition)
        }
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


// ===============================================================================================
//
// Conversion to a Python dict.
//
// ===============================================================================================

impl ToPyObject for ffi::Mixture {
    fn to_object(&self, py: Python) -> PyObject {
        if self.properties.density <= 0.0 {
            py.None()
        } else {
            let state = State::try_from(self.properties.state)
                .map_or_else(
                    |err| err.to_owned(),
                    |state| state.to_str().to_owned(),
                )
                .to_string();
            let composition: Vec<_> = self.components.iter()
                .map(|component| (&component.name, component.weight))
                .collect();

            Namespace::new(py, &[
                ("density", self.properties.density.into_py(py)),
                ("state", state.into_py(py)),
                ("composition", composition.into_py(py)),
            ]).unwrap().unbind()
        }
    }
}
