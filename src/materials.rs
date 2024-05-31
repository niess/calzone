use crate::error::variant_error;
use crate::utils::{Extractor, Property, Tag, TryFromBound};
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use super::cxx::ffi;

mod hash;


#[pyfunction]
#[pyo3(signature = (**kwargs,))]
pub fn elements(kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
    if let Some(kwargs) = kwargs {
        let elements = Vec::<ffi::Element>::try_from_dict(&Tag::empty(), kwargs)?;
        for element in elements {
            ffi::add_element(&element)
                .to_result()?;
        }
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (**kwargs,))]
pub fn molecules(kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
    if let Some(kwargs) = kwargs {
        let molecules = Vec::<ffi::Molecule>::try_from_dict(&Tag::empty(), kwargs)?;
        for molecule in molecules {
            ffi::add_molecule(&molecule)
                .to_result()?;
        }
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (**kwargs,))]
pub fn mixtures(kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
    if let Some(kwargs) = kwargs {
        let mixtures = Vec::<ffi::Mixture>::try_from_dict(&Tag::empty(), kwargs)?;
        for mixture in mixtures {
            ffi::add_mixture(&mixture)
                .to_result()?;
        }
    }
    Ok(())
}


// ===============================================================================================
//
// Conversions (from a Python dict).
//
// ===============================================================================================

impl TryFromBound for ffi::Element {
    const TYPE_NAME: &'static str = "element";

    #[allow(non_snake_case)]
    fn try_from_dict<'py>(tag: &Tag, value: &Bound<'py, PyDict>) -> PyResult<Self> {
        const EXTRACTOR: Extractor<3> = Extractor::new(ffi::Element::TYPE_NAME, [
            Property::required_f64("Z"),
            Property::required_f64("A"),
            Property::optional_str("symbol"),
        ]);

        let [Z, A, symbol] = EXTRACTOR.extract(tag.path().as_ref(), value)?;
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
    const TYPE_NAME: &'static str = "molecule";

    fn try_from_dict<'py>(tag: &Tag, value: &Bound<'py, PyDict>) -> PyResult<Self> {
        const EXTRACTOR: Extractor<3> = Extractor::new(ffi::Molecule::TYPE_NAME, [
            Property::required_f64("density"),
            Property::required_dict("composition"),
            Property::optional_str("state"),
        ]);

        let (properties, composition) = try_into_properties(
            &EXTRACTOR, ffi::Molecule::TYPE_NAME, tag, value
        )?;
        let components = Vec::<ffi::MoleculeComponent>::try_from_dict(
            tag, &composition
        )?;

        let molecule = Self::new(properties, components);
        Ok(molecule)
    }
}

impl TryFromBound for ffi::Mixture {
    const TYPE_NAME: &'static str = "mixture";

    fn try_from_dict<'py>(tag: &Tag, value: &Bound<'py, PyDict>) -> PyResult<Self> {
        const EXTRACTOR: Extractor<3> = Extractor::new(ffi::Mixture::TYPE_NAME, [
            Property::required_f64("density"),
            Property::required_dict("composition"),
            Property::optional_str("state"),
        ]);

        let (properties, composition) = try_into_properties(
            &EXTRACTOR, ffi::Mixture::TYPE_NAME, tag, value
        )?;
        let components = Vec::<ffi::MixtureComponent>::try_from_dict(
            tag, &composition
        )?;

        let mixture = Self::new(properties, components);
        Ok(mixture)
    }
}

fn try_into_properties<'py>(
    extractor: &Extractor<3>,
    context: &str,
    tag: &Tag,
    value: &Bound<'py, PyDict>
) -> PyResult<(ffi::MaterialProperties, Bound<'py, PyDict>)> {
    let path = tag.path();
    let [density, composition, state] = extractor.extract(path.as_ref(), value)?;

    let state: ffi::G4State = if state.is_none() {
        ffi::G4State::kStateUndefined
    } else {
        let state: String = state.into();
        let state = State::from_str(state.as_str())
            .map_err(|options| {
                let message = format!("bad 'state' for {} '{}'", context, path);
                variant_error(message.as_str(), state.as_str(), options)
            })?;
        state.into()
    };
    let properties = ffi::MaterialProperties {
        name: tag.name().to_string(),
        density: density.into(),
        state,
    };

    let composition: Bound<PyDict> = composition.into();
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
    const TYPE_NAME: &'static str = "molecule component";

    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let property = Property::required_u32("weight");
        let context = format!("{} '{}'", ffi::MoleculeComponent::TYPE_NAME, tag.path());
        let weight = property.extract(context.as_str(), value)?;

        let component = Self {
            name: tag.name().to_string(),
            weight: weight.into(),
        };
        Ok(component)
    }
}

impl TryFromBound for ffi::MixtureComponent {
    const TYPE_NAME: &'static str = "mixture component";

    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let property = Property::required_f64("weight");
        let context = format!("{} '{}'", ffi::MixtureComponent::TYPE_NAME, tag.path());
        let weight = property.extract(context.as_str(), value)?;

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
