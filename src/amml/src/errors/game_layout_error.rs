/// A game entry the ghost needs is missing, or a mirror does not fit the build.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum GameLayoutError {
    #[error("{entry} did not resolve")]
    Missing { entry: &'static str },
    #[error("{entry} is {found:#x}, the mirror needs {expected:#x}")]
    Misfit { entry: &'static str, expected: u64, found: u64 },
}
