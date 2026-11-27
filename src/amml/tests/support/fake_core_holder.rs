use alaydriem_stratum::traits::{UseCore, UseCoreHolder};

use super::FakeUseCore;

/// A holder of a fake core, as the runtime's core state holds the running core.
pub struct FakeCoreHolder {
    pub core: FakeUseCore,
}

impl UseCoreHolder for FakeCoreHolder {
    fn use_core(&mut self) -> Option<&mut dyn UseCore> {
        Some(&mut self.core)
    }
}
