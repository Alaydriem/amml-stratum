// The `import` block of the config file.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct ImportSettings {
    pub max_file_mb: u64,
    pub inbox_poll_seconds: f32,
    pub queue: usize,
}

impl Default for ImportSettings {
    fn default() -> Self {
        Self { max_file_mb: 256, inbox_poll_seconds: 2.0, queue: 64 }
    }
}
