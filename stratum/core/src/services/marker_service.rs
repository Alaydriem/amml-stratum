use crate::config::MarkerConfig;
use crate::errors::MarkerError;
use crate::services::MarkerControls;
use crate::structs::action::HudAction;
use crate::structs::library::{Library, NameLimit, ProjectId, StructureId};
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

    pub fn assign(&mut self, id: MarkerId, target: MarkerTarget) -> Result<(), MarkerError> {
        let index = self.index(id)?;
        self.set.markers[index].target = target;
        self.dirty = true;
        Ok(())
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
    pub fn apply_hud(&mut self, action: HudAction, layers: u32, structures: u32) -> Result<(), MarkerError> {
        let id = self.set.active.ok_or(MarkerError::NotFound)?;
        let index = self.index(id)?;
        MarkerControls::apply(&mut self.set.markers[index], action, layers, structures);
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
            MarkerTarget::Project(project) => library.projects.iter().any(|entry| entry.id == project),
        };
        if found { TargetState::Ok } else { TargetState::Missing }
    }

    pub fn uses_of_structure(&self, id: StructureId) -> Vec<MarkerId> {
        self.uses(MarkerTarget::Structure(id))
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
