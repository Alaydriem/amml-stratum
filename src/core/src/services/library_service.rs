use crate::config::LibraryConfig;
use crate::errors::LibraryError;
use crate::structs::action::MaterialScope;
use crate::structs::layout::GridCell;
use crate::structs::library::{
    Library, NameLimit, NewStructure, Project, ProjectId, ProjectKind, ProjectOrder, StructureEntry, StructureId,
};
use crate::structs::material::{GatheredItem, ItemId, MaterialList};

// Holds the library and applies every change to it. Capacity: `max_structures` and
// `max_projects` from config; an add past either is refused, nothing is evicted. The ticks of one
// material list hold at most `MAX_GATHERED` items; a new tick past it is refused.
pub struct LibraryService {
    library: Library,
    config: LibraryConfig,
    dirty: bool,
    order: ProjectOrder,
}

impl LibraryService {
    pub const MAX_GATHERED: usize = 4096;

    pub fn new(library: Library, config: LibraryConfig) -> Self {
        let mut order = ProjectOrder::new();
        order.rebuild(&library);
        Self { library, config, dirty: false, order }
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn order(&self) -> &ProjectOrder {
        &self.order
    }

    pub fn add_structure(&mut self, entry: NewStructure) -> Result<StructureId, LibraryError> {
        let name = Self::clean_name(&entry.display_name)?;
        Self::check_file_name(&entry.file_name)?;
        let full = LibraryError::Full { capacity: self.config.max_structures };
        if self.library.structures.len() >= self.config.max_structures {
            return Err(full);
        }
        if let Some(project) = entry.project {
            self.project_index(project)?;
        }
        let id = self.library.next_structure_id;
        self.library.next_structure_id = id.checked_add(1).ok_or(full)?;
        let display_name = self.unused_name(&name, entry.project);
        self.library.structures.push(StructureEntry {
            id: StructureId(id),
            display_name,
            file_name: entry.file_name,
            project: entry.project,
            size: entry.size,
            block_count: entry.block_count,
            world_origin: entry.world_origin,
            grid_cell: None,
            gathered: Vec::new(),
        });
        self.touched();
        Ok(StructureId(id))
    }

    pub fn rename_structure(&mut self, id: StructureId, name: &str) -> Result<(), LibraryError> {
        let name = Self::clean_name(name)?;
        let index = self.structure_index(id)?;
        self.library.structures[index].display_name = name;
        self.touched();
        Ok(())
    }

    pub fn move_structure(&mut self, id: StructureId, project: Option<ProjectId>) -> Result<(), LibraryError> {
        let index = self.structure_index(id)?;
        if let Some(project) = project {
            self.project_index(project)?;
        }
        self.library.structures[index].project = project;
        self.touched();
        Ok(())
    }

    pub fn set_grid_cell(&mut self, id: StructureId, cell: Option<GridCell>) -> Result<(), LibraryError> {
        let index = self.structure_index(id)?;
        self.library.structures[index].grid_cell = cell;
        self.touched();
        Ok(())
    }

    // Returns the removed entry so the caller can remove its file.
    pub fn delete_structure(&mut self, id: StructureId) -> Result<StructureEntry, LibraryError> {
        let index = self.structure_index(id)?;
        let removed = self.library.structures.remove(index);
        self.touched();
        Ok(removed)
    }

    pub fn create_project(&mut self, name: &str) -> Result<ProjectId, LibraryError> {
        let name = Self::clean_name(name)?;
        let full = LibraryError::Full { capacity: self.config.max_projects };
        if self.library.projects.len() >= self.config.max_projects {
            return Err(full);
        }
        let id = self.library.next_project_id;
        self.library.next_project_id = id.checked_add(1).ok_or(full)?;
        self.library.projects.push(Project {
            id: ProjectId(id),
            name,
            kind: ProjectKind::Normal,
            gathered: Vec::new(),
        });
        self.touched();
        Ok(ProjectId(id))
    }

    pub fn rename_project(&mut self, id: ProjectId, name: &str) -> Result<(), LibraryError> {
        let name = Self::clean_name(name)?;
        let index = self.project_index(id)?;
        self.library.projects[index].name = name;
        self.touched();
        Ok(())
    }

    pub fn set_project_kind(&mut self, id: ProjectId, kind: ProjectKind) -> Result<(), LibraryError> {
        let index = self.project_index(id)?;
        self.library.projects[index].kind = kind;
        self.touched();
        Ok(())
    }

    // The structures of the project become Unsorted.
    pub fn delete_project(&mut self, id: ProjectId) -> Result<(), LibraryError> {
        let index = self.project_index(id)?;
        self.library.projects.remove(index);
        for entry in &mut self.library.structures {
            if entry.project == Some(id) {
                entry.project = None;
            }
        }
        self.touched();
        Ok(())
    }

    // The ticks of a material list; empty when its project or structure is gone.
    pub fn gathered(&self, scope: MaterialScope) -> &[GatheredItem] {
        match scope {
            MaterialScope::Project(id) => {
                self.library.projects.iter().find(|project| project.id == id).map_or(&[], |p| p.gathered.as_slice())
            }
            MaterialScope::Structure(id) => {
                self.library.structures.iter().find(|entry| entry.id == id).map_or(&[], |e| e.gathered.as_slice())
            }
        }
    }

    // `Some(count)` ticks the item with the count the list needs now; `None` clears the tick.
    pub fn set_gathered(&mut self, scope: MaterialScope, item: ItemId, count: Option<u64>) -> Result<(), LibraryError> {
        let ticks = self.gathered_mut(scope)?;
        let found = ticks.iter().position(|tick| tick.item == item);
        let changed = match (found, count) {
            (Some(index), Some(count)) => {
                let changed = ticks[index].count != count;
                ticks[index].count = count;
                changed
            }
            (Some(index), None) => {
                ticks.remove(index);
                true
            }
            (None, Some(count)) => {
                if ticks.len() >= Self::MAX_GATHERED {
                    return Err(LibraryError::Full { capacity: Self::MAX_GATHERED });
                }
                ticks.push(GatheredItem { item, count });
                true
            }
            (None, None) => false,
        };
        if changed {
            self.touched();
        }
        Ok(())
    }

    // Drops the ticks whose item left `list` or whose needed count rose past the ticked count.
    // True when a tick was dropped.
    pub fn reconcile_gathered(&mut self, scope: MaterialScope, list: &MaterialList) -> bool {
        let Ok(ticks) = self.gathered_mut(scope) else {
            return false;
        };
        let before = ticks.len();
        ticks.retain(|tick| list.lines().iter().any(|line| line.item == tick.item && line.count <= tick.count));
        let dropped = ticks.len() != before;
        if dropped {
            self.touched();
        }
        dropped
    }

    // Sorted by display name, ignoring case. `None` lists the Unsorted structures.
    pub fn structures_in(&self, project: Option<ProjectId>) -> Vec<&StructureEntry> {
        let mut found: Vec<&StructureEntry> =
            self.library.structures.iter().filter(|entry| entry.project == project).collect();
        found.sort_by_cached_key(|entry| (entry.display_name.to_lowercase(), entry.id));
        found
    }

    fn touched(&mut self) {
        self.order.rebuild(&self.library);
        self.dirty = true;
    }

    fn gathered_mut(&mut self, scope: MaterialScope) -> Result<&mut Vec<GatheredItem>, LibraryError> {
        match scope {
            MaterialScope::Project(id) => {
                let index = self.project_index(id)?;
                Ok(&mut self.library.projects[index].gathered)
            }
            MaterialScope::Structure(id) => {
                let index = self.structure_index(id)?;
                Ok(&mut self.library.structures[index].gathered)
            }
        }
    }

    fn structure_index(&self, id: StructureId) -> Result<usize, LibraryError> {
        self.library.structures.iter().position(|entry| entry.id == id).ok_or(LibraryError::NotFound)
    }

    fn project_index(&self, id: ProjectId) -> Result<usize, LibraryError> {
        self.library.projects.iter().position(|project| project.id == id).ok_or(LibraryError::NotFound)
    }

    // `name`, or `name (2)`, `name (3)` ... for the first one no structure in the project uses.
    fn unused_name(&self, name: &str, project: Option<ProjectId>) -> String {
        let taken = |candidate: &str| {
            self.library.structures.iter().any(|entry| entry.project == project && entry.display_name == candidate)
        };
        if !taken(name) {
            return name.to_string();
        }
        // One more attempt than there are structures always finds a free suffix.
        for number in 2..=self.library.structures.len() + 2 {
            let suffix = format!(" ({number})");
            let keep = NameLimit::MAX_CHARS.saturating_sub(suffix.chars().count());
            let candidate: String = name.chars().take(keep).collect::<String>() + &suffix;
            if !taken(&candidate) {
                return candidate;
            }
        }
        name.to_string()
    }

    fn check_file_name(name: &str) -> Result<(), LibraryError> {
        let plain = !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', ':']);
        if plain { Ok(()) } else { Err(LibraryError::BadFileName) }
    }

    fn clean_name(name: &str) -> Result<String, LibraryError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LibraryError::NameEmpty);
        }
        if name.chars().count() > NameLimit::MAX_CHARS {
            return Err(LibraryError::NameTooLong);
        }
        Ok(name.to_string())
    }
}
