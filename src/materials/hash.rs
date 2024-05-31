use std::hash::{DefaultHasher, Hash, Hasher};
use super::ffi;


impl Hash for ffi::MaterialProperties {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        unsafe { std::mem::transmute::<f64, u64>(self.density) }.hash(state);
        unsafe { std::mem::transmute::<ffi::G4State, u32>(self.state) }.hash(state);
    }
}

impl Hash for ffi::MixtureComponent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        unsafe { std::mem::transmute::<f64, u64>(self.weight) }.hash(state);
    }
}

fn get_hash<H: Hash>(slf: &H) -> u64 { // This is a workaround, since shared types do not support
                                       // custom traits.
    let mut hasher = DefaultHasher::new();
    slf.hash(&mut hasher);
    hasher.finish()
}

impl ffi::Mixture {
    #[inline]
    pub fn get_hash(&self) -> u64 {
        get_hash(self)
    }
}

impl ffi::Molecule {
    #[inline]
    pub fn get_hash(&self) -> u64 {
        get_hash(self)
    }
}
