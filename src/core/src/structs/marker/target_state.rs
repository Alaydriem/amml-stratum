// Whether the structure or project a marker shows is still in the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetState {
    Ok,
    Missing,
}
