// The game's answer to a `BuildKindsRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildKindsResult {
    // The block names in the box, each once. Air may be left out.
    Kinds(Vec<String>),
    // Part of the box is not loaded.
    NotLoaded,
    // The box holds more kinds than `BuildConfig::kinds_capacity`.
    TooMany,
}
