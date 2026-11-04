use crate::config::MarkerConfig;
use crate::errors::MarkerError;
use crate::services::MarkerControls;
use crate::structs::action::HudAction;
use crate::structs::library::{Library, NameLimit, ProjectId, ProjectKind, ProjectOrder, StructureId};
use crate::structs::marker::{Marker, MarkerId, MarkerSet, MarkerTarget, MarkerView, Rotation, TargetState};

// Holds the markers of one world and applies every change to them. Capacity: `max_per_world`
// from config; a place past it is refused, nothing is evicted.
pub struct MarkerService {
    set: MarkerSet,
    config: MarkerConfig,
    dirty: bool,
}

impl MarkerService {
    pub fn new(set: MarkerSet, config: MarkerConfig) -> Self {
        Self { set, config, dirty: false }
    }

    pub fn set(&self) -> &MarkerSet {
        &self.set
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn place(&mut self, name: &str, origin: [i32; 3], target: MarkerTarget) -> Result<MarkerId, MarkerError> {
        let name = Self::clean_name(name)?;
        let full = MarkerError::Full { capacity: self.config.max_per_world };
        if self.set.markers.len() >= self.config.max_per_world {
            return Err(full);
        }
        let id = self.set.next_id;
        self.set.next_id = id.checked_add(1).ok_or(full)?;
        self.set.markers.push(Marker {
            id: MarkerId(id),
            name,
            origin,
            rotation: Rotation::R0,
            target,
            view: MarkerView::default(),
        });
        self.dirty = true;
        Ok(MarkerId(id))
    }

    // Makes the marker the only active one.
    pub fn activate(&mut self, id: MarkerId) -> Result<(), MarkerError> {
        self.index(id)?;
        self.set.active = Some(id);
        self.dirty = true;
        Ok(())
    }

    pub fn deactivate(&mut self) {
        if self.set.active.take().is_some() {
            self.dirty = true;
        }
    }

    pub fn rename(&mut self, id: MarkerId, name: &str) -> Result<(), MarkerError> {
        let name = Self::clean_name(name)?;
        let index = self.index(id)?;
        self.set.markers[index].name = name;
        self.dirty = true;
        Ok(())
    }

    // Gives the marker a new target. A marker that cycles through a normal project keeps its place
    // when the target is that project or one of its structures, and a structure of the project is a
    // step. Any other target clears the saved part views.
    pub fn assign(
        &mut self,
        id: MarkerId,
        target: MarkerTarget,
        library: &Library,
        order: &ProjectOrder,
    ) -> Result<(), MarkerError> {
        let index = self.index(id)?;
        let marker = &mut self.set.markers[index];
        let own = match marker.target {
            MarkerTarget::Project(project) if Self::is_normal(library, project) => Some(project),
            _ => None,
        };
        match target {
            MarkerTarget::Project(project) if Self::is_normal(library, project) => {
                if own == Some(project) {
                    return Ok(());
                }
                marker.target = target;
                marker.view.forget_parts();
                marker.view.current = None;
                if let Some(first) = order.of(project).first() {
                    marker.view.switch_to(*first);
                }
            }
            MarkerTarget::Structure(structure) => match Self::normal_project_of(library, structure) {
                Some(project) if own == Some(project) => marker.view.switch_to(structure),
                Some(project) => {
                    marker.target = MarkerTarget::Project(project);
                    marker.view.forget_parts();
                    marker.view.current = None;
                    marker.view.switch_to(structure);
                }
                None => Self::retarget(marker, target),
            },
            MarkerTarget::Project(_) => Self::retarget(marker, target),
        }
        self.dirty = true;
        Ok(())
    }

    fn retarget(marker: &mut Marker, target: MarkerTarget) {
        marker.target = target;
        marker.view.forget_parts();
        marker.view.current = None;
    }

    // Brings every marker in line with the library: a structure marker in a normal project starts to
    // cycle, a cycling marker whose structure left its project stops, and a marker with no current
    // structure opens on one. Returns true when any marker changed.
    pub fn reconcile(&mut self, library: &Library, order: &ProjectOrder) -> bool {
        let mut any = false;
        for marker in &mut self.set.markers {
            any |= Self::reconcile_marker(marker, library, order);
        }
        if any {
            self.dirty = true;
        }
        any
    }

    fn reconcile_marker(marker: &mut Marker, library: &Library, order: &ProjectOrder) -> bool {
        // The index of a file from before `current` applies to the first reconcile only.
        let legacy_selected = marker.view.legacy_selected.take();
        match marker.target {
            MarkerTarget::Structure(structure) => {
                let Some(project) = Self::normal_project_of(library, structure) else {
                    return false;
                };
                marker.target = MarkerTarget::Project(project);
                marker.view.current = Some(structure);
                marker.view.forget_parts();
                marker.view.reserve_parts();
                true
            }
            MarkerTarget::Project(project) if Self::is_normal(library, project) => {
                let list = order.of(project);
                let mut changed = false;
                match marker.view.current {
                    None => {
                        let by_import = legacy_selected.and_then(|index| {
                            let index = usize::try_from(index).unwrap_or(usize::MAX);
                            library
                                .structures
                                .iter()
                                .filter(|entry| entry.project == Some(project))
                                .nth(index)
                                .map(|entry| entry.id)
                        });
                        marker.view.current = by_import.or_else(|| list.first().copied());
                        changed = marker.view.current.is_some();
                    }
                    Some(current) if !list.contains(&current) => {
                        if library.structures.iter().any(|entry| entry.id == current) {
                            marker.view.forget_parts();
                            match Self::normal_project_of(library, current) {
                                Some(moved_to) => {
                                    marker.target = MarkerTarget::Project(moved_to);
                                    marker.view.reserve_parts();
                                }
                                None => {
                                    marker.target = MarkerTarget::Structure(current);
                                    marker.view.current = None;
                                }
                            }
                            return true;
                        }
                        marker.view.current = None;
                        if let Some(first) = list.first() {
                            marker.view.switch_to(*first);
                        }
                        changed = true;
                    }
                    Some(_) => {}
                }
                marker.view.reserve_parts();
                let kept = marker.view.parts.len();
                marker.view.keep_parts(list);
                changed || marker.view.parts.len() != kept
            }
            MarkerTarget::Project(_) => {
                let Some(current) = marker.view.current else {
                    return false;
                };
                if !library.structures.iter().any(|entry| entry.id == current) {
                    return false;
                }
                marker.target = MarkerTarget::Structure(current);
                marker.view.current = None;
                marker.view.forget_parts();
                true
            }
        }
    }

    // Call before the library deletes `id`. A marker that cycles and shows `id` moves to the next
    // structure by name when the project has another one, and every marker drops its part for `id`.
    pub fn structure_removing(&mut self, id: StructureId, order: &ProjectOrder) {
        let mut any = false;
        for marker in &mut self.set.markers {
            let MarkerTarget::Project(project) = marker.target else {
                continue;
            };
            if marker.view.current == Some(id)
                && let Some(next) = order.step(project, id, true).filter(|next| *next != id)
            {
                marker.view.switch_to(next);
                any = true;
            }
            let kept = marker.view.parts.len();
            marker.view.parts.retain(|part| part.structure != id);
            any |= marker.view.parts.len() != kept;
        }
        if any {
            self.dirty = true;
        }
    }

    fn is_normal(library: &Library, project: ProjectId) -> bool {
        library.projects.iter().any(|entry| entry.id == project && entry.kind == ProjectKind::Normal)
    }

    // The normal project that holds the structure, if it is in one.
    fn normal_project_of(library: &Library, structure: StructureId) -> Option<ProjectId> {
        let project = library.structures.iter().find(|entry| entry.id == structure)?.project?;
        Self::is_normal(library, project).then_some(project)
    }

    // Deleting the active marker leaves no marker active.
    pub fn delete(&mut self, id: MarkerId) -> Result<(), MarkerError> {
        let index = self.index(id)?;
        self.set.markers.remove(index);
        if self.set.active == Some(id) {
            self.set.active = None;
        }
        self.dirty = true;
        Ok(())
    }

    // Applies a HUD action to the active marker.
    pub fn apply_hud(&mut self, action: HudAction, layers: u32, order: &[StructureId]) -> Result<(), MarkerError> {
        let id = self.set.active.ok_or(MarkerError::NotFound)?;
        let index = self.index(id)?;
        MarkerControls::apply(&mut self.set.markers[index], action, layers, order);
        self.dirty = true;
        Ok(())
    }

    pub fn active(&self) -> Option<&Marker> {
        let id = self.set.active?;
        self.set.markers.iter().find(|marker| marker.id == id)
    }

    pub fn markers(&self) -> &[Marker] {
        &self.set.markers
    }

    // `Missing` when the marker, or the structure or project it shows, is not there.
    pub fn target_state(&self, id: MarkerId, library: &Library) -> TargetState {
        let Some(marker) = self.set.markers.iter().find(|marker| marker.id == id) else {
            return TargetState::Missing;
        };
        let found = match marker.target {
            MarkerTarget::Structure(structure) => library.structures.iter().any(|entry| entry.id == structure),
            MarkerTarget::Project(project) => match library.projects.iter().find(|entry| entry.id == project) {
                None => false,
                Some(entry) if entry.kind == ProjectKind::Normal => marker
                    .view
                    .current
                    .is_some_and(|current| library.structures.iter().any(|entry| entry.id == current)),
                Some(_) => true,
            },
        };
        if found { TargetState::Ok } else { TargetState::Missing }
    }

    pub fn uses_of_structure(&self, id: StructureId) -> Vec<MarkerId> {
        self.set
            .markers
            .iter()
            .filter(|marker| {
                marker.target == MarkerTarget::Structure(id)
                    || (matches!(marker.target, MarkerTarget::Project(_)) && marker.view.current == Some(id))
            })
            .map(|marker| marker.id)
            .collect()
    }

    pub fn uses_of_project(&self, id: ProjectId) -> Vec<MarkerId> {
        self.uses(MarkerTarget::Project(id))
    }

    fn uses(&self, target: MarkerTarget) -> Vec<MarkerId> {
        self.set.markers.iter().filter(|marker| marker.target == target).map(|marker| marker.id).collect()
    }

    fn index(&self, id: MarkerId) -> Result<usize, MarkerError> {
        self.set.markers.iter().position(|marker| marker.id == id).ok_or(MarkerError::NotFound)
    }

    fn clean_name(name: &str) -> Result<String, MarkerError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(MarkerError::NameEmpty);
        }
        if name.chars().count() > NameLimit::MAX_CHARS {
            return Err(MarkerError::NameTooLong);
        }
        Ok(name.to_string())
    }
}
