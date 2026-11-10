use super::{Project, StructureEntry};

// Saved library data. Logic lives in `LibraryService`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Library {
    pub structures: Vec<StructureEntry>,
    pub projects: Vec<Project>,
    pub(crate) next_structure_id: u64,
    pub(crate) next_project_id: u64,
}

impl Library {
    pub fn new() -> Self {
        Self { structures: Vec::new(), projects: Vec::new(), next_structure_id: 1, next_project_id: 1 }
    }

    // Raises both id counters past every id in use, so a file with missing counters cannot reuse an id.
    pub(crate) fn clamp_counters(&mut self) {
        let structures = self.structures.iter().map(|entry| entry.id.0).max().map_or(1, |max| max.saturating_add(1));
        let projects = self.projects.iter().map(|project| project.id.0).max().map_or(1, |max| max.saturating_add(1));
        self.next_structure_id = self.next_structure_id.max(structures);
        self.next_project_id = self.next_project_id.max(projects);
    }
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}
