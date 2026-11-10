use std::ffi::c_void;

/// What one easy place step reads from the client before it asks the core: the live client, its local player and
/// region (null outside a world), whether the use key is down with no menu showing, and the steady clock in seconds.
#[derive(Clone, Copy, Debug)]
pub struct UseFrame {
    pub client: *mut c_void,
    pub player: *mut c_void,
    pub region: *mut c_void,
    pub key_held: bool,
    pub now: f64,
}
