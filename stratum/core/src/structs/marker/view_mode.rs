// Whether a marker shows one layer of its structure or all of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewMode {
    Layer,
    Whole,
}
