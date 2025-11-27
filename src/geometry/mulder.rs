use std::ffi::c_void;


#[link(name = "geant4")]
extern "C" {
    pub fn g4mulder_initialise() -> Module;
}

#[repr(C)]
pub struct Module {
    element: *mut c_void,
    geometry: *mut c_void,
    material: *mut c_void,
}

#[no_mangle]
pub unsafe extern "C" fn mulder_initialise() -> Module {
    g4mulder_initialise()
}
