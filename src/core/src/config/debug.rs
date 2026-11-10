// The `debug` block of the config file.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct DebugConfig {
    // Logs one telemetry line every 2 seconds.
    pub telemetry: bool,
}
