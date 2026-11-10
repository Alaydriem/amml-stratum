use std::collections::HashMap;

use super::{Library, ProjectId, StructureId};

// Structure ids of each project in name order, ignoring case, then by id. Bound: one list per
// project and one entry per structure, so `max_projects` and `max_structures` bound it. Lists
// are rebuilt from the library, never appended.
#[derive(Debug, Default)]
pub struct ProjectOrder {
    lists: HashMap<ProjectId, Vec<StructureId>>,
}

impl ProjectOrder {
    pub fn new() -> Self {
        Self { lists: HashMap::new() }
    }

    pub fn rebuild(&mut self, library: &Library) {
        self.lists.clear();
        let mut keyed: HashMap<ProjectId, Vec<(String, StructureId)>> = HashMap::new();
        for entry in &library.structures {
            if let Some(project) = entry.project {
                keyed.entry(project).or_default().push((entry.display_name.to_lowercase(), entry.id));
            }
        }
        for (project, mut list) in keyed {
            list.sort();
            self.lists.insert(project, list.into_iter().map(|(_, id)| id).collect());
        }
    }

    // Empty for an unknown project.
    pub fn of(&self, project: ProjectId) -> &[StructureId] {
        self.lists.get(&project).map_or(&[], Vec::as_slice)
    }

    pub fn position(&self, project: ProjectId, id: StructureId) -> Option<usize> {
        self.of(project).iter().position(|candidate| *candidate == id)
    }

    // The next (or previous) structure of the project, wrapping at the ends. `None` when `from`
    // is not in the project.
    pub fn step(&self, project: ProjectId, from: StructureId, forward: bool) -> Option<StructureId> {
        let list = self.of(project);
        let at = self.position(project, from)?;
        let next = if forward { (at + 1) % list.len() } else { (at + list.len() - 1) % list.len() };
        list.get(next).copied()
    }
}
