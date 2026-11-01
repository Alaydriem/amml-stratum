#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct LibraryConfig {
    pub max_structures: usize,
    pub max_projects: usize,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self { max_structures: 4096, max_projects: 512 }
    }
}
