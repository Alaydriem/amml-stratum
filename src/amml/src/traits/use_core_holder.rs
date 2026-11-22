use crate::traits::UseCore;

// What holds the core a use call asks, behind the caller's lock: the running core, or None while no core runs
pub trait UseCoreHolder {
    fn use_core(&mut self) -> Option<&mut dyn UseCore>;
}
