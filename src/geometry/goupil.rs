use std::ffi::c_void;


#[link(name = "geant4")]
extern "C" {
    pub fn g4goupil_initialise() -> Interface;
}

#[repr(C)]
pub struct Interface {
    new_geometry_definition: *mut c_void,
    new_geometry_tracer: *mut c_void,
}

#[no_mangle]
pub unsafe extern "C" fn goupil_initialise() -> Interface {
    g4goupil_initialise()
}
