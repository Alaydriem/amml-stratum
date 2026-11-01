use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::errors::GhostError;
use crate::services::LayoutSolver;
use crate::structs::ghost::PlacedStructure;
use crate::structs::ghost::{GhostBuild, GhostPalette, GhostPiece};
use crate::structs::layout::LayoutIssue;
use crate::structs::library::{Library, ProjectId, ProjectKind, StructureEntry, StructureId};
use crate::structs::marker::{Marker, MarkerTarget, Rotation};
use crate::structs::structure::{BlockLayer, BlockState, SparseLayer, Structure, StructureSize};

// Turns a marker and the loaded structures into one world-space build with a shared palette.
pub struct GhostAssembler;

impl GhostAssembler {
    // The structures a marker needs loaded: itself for a structure target, the selected structure
    // of a normal project, every piece of a big build.
    pub fn needed(marker: &Marker, library: &Library) -> Result<Vec<StructureId>, GhostError> {
        match marker.target {
            MarkerTarget::Structure(id) => Ok(vec![id]),
            MarkerTarget::Project(project) => {
                let (kind, entries) = Self::project_entries(library, project)?;
                match kind {
                    ProjectKind::Normal => Self::selected(marker, &entries).map(|entry| vec![entry.id]),
                    ProjectKind::BigBuild => {
                        if entries.is_empty() {
                            return Err(GhostError::Layout(LayoutIssue::NoPieces));
                        }
                        LayoutSolver::solve(&entries).map_err(GhostError::Layout)?;
                        Ok(entries.iter().map(|entry| entry.id).collect())
                    }
                }
            }
        }
    }

    // `world_max` of the build is inclusive. The build turns by `marker.rotation` around the cell at
    // `marker.origin`.
    pub fn assemble(
        marker: &Marker,
        library: &Library,
        loaded: &HashMap<StructureId, Arc<Structure>>,
        revision: u64,
    ) -> Result<GhostBuild, GhostError> {
        let placed = Self::placed(marker, library, loaded)?;
        let turned: Vec<Arc<Structure>> = placed
            .iter()
            .map(|piece| match marker.rotation {
                Rotation::R0 => Arc::clone(&piece.structure),
                rotation => Arc::new(Self::turn(&piece.structure, rotation)),
            })
            .collect();
        Self::build(&placed, &turned, marker.origin, marker.rotation, revision, None)
    }

    // Each structure of the build with the world position of its cell (0, 0, 0) before the turn.
    pub fn placed(
        marker: &Marker,
        library: &Library,
        loaded: &HashMap<StructureId, Arc<Structure>>,
    ) -> Result<Vec<PlacedStructure>, GhostError> {
        let mut placed: Vec<PlacedStructure> = Vec::new();
        match marker.target {
            MarkerTarget::Structure(id) => {
                placed.push(PlacedStructure { structure: Self::fetch(loaded, id)?, origin: marker.origin });
            }
            MarkerTarget::Project(project) => {
                let (kind, entries) = Self::project_entries(library, project)?;
                match kind {
                    ProjectKind::Normal => {
                        let entry = Self::selected(marker, &entries)?;
                        placed
                            .push(PlacedStructure { structure: Self::fetch(loaded, entry.id)?, origin: marker.origin });
                    }
                    ProjectKind::BigBuild => {
                        let layout = LayoutSolver::solve(&entries).map_err(GhostError::Layout)?;
                        for placement in &layout.pieces {
                            let structure = Self::fetch(loaded, placement.structure)?;
                            let mut origin = marker.origin;
                            for (axis, value) in origin.iter_mut().enumerate() {
                                *value = value
                                    .checked_add(placement.offset[axis])
                                    .ok_or(GhostError::Layout(LayoutIssue::TooLarge))?;
                            }
                            placed.push(PlacedStructure { structure, origin });
                        }
                    }
                }
            }
        }
        Ok(placed)
    }

    // The kind of a project and its structures in library order.
    fn project_entries(
        library: &Library,
        project: ProjectId,
    ) -> Result<(ProjectKind, Vec<&StructureEntry>), GhostError> {
        let kind = library
            .projects
            .iter()
            .find(|candidate| candidate.id == project)
            .map(|candidate| candidate.kind)
            .ok_or(GhostError::MissingTarget)?;
        let entries = library.structures.iter().filter(|entry| entry.project == Some(project)).collect();
        Ok((kind, entries))
    }

    // The normal project's selected structure, `view.selected` clamped to the project's structures.
    fn selected<'a>(marker: &Marker, entries: &[&'a StructureEntry]) -> Result<&'a StructureEntry, GhostError> {
        let last = entries.len().checked_sub(1).ok_or(GhostError::MissingTarget)?;
        let index = usize::try_from(marker.view.selected).unwrap_or(usize::MAX).min(last);
        entries.get(index).copied().ok_or(GhostError::MissingTarget)
    }

    fn fetch(loaded: &HashMap<StructureId, Arc<Structure>>, id: StructureId) -> Result<Arc<Structure>, GhostError> {
        loaded.get(&id).cloned().ok_or(GhostError::NotLoaded(id))
    }

    // Builds the shared palette and each piece's remap, then the build's world box. Each piece is
    // placed turned by `rotation` around the cell at `pivot`, so a big build turns as one;
    // `turned[n]` is `placed[n]`'s structure turned by `rotation` (`turn`), or the same structure at
    // R0. `reuse` is a build of the same structures in the same order: its palette and remaps are
    // kept, so a move or a turn reads no cell here.
    pub fn build(
        placed: &[PlacedStructure],
        turned: &[Arc<Structure>],
        pivot: [i32; 3],
        rotation: Rotation,
        revision: u64,
        reuse: Option<&GhostBuild>,
    ) -> Result<GhostBuild, GhostError> {
        let reuse = reuse.filter(|build| build.pieces.len() == placed.len());
        let mut palette = reuse.map(|build| build.palette.clone()).unwrap_or_default();
        let mut ids: HashMap<BlockState, u16> = HashMap::new();
        let mut pieces = Vec::with_capacity(placed.len());
        let mut world_min = [i64::MAX; 3];
        let mut world_max = [i64::MIN; 3];
        for (index, (PlacedStructure { structure, origin }, turned)) in placed.iter().zip(turned).enumerate() {
            let remap = match reuse.and_then(|build| build.pieces.get(index)) {
                Some(piece) => piece.remap.clone(),
                None => Self::remap(structure, &mut palette, &mut ids)?,
            };
            let size = [structure.size.x, structure.size.y, structure.size.z];
            let offset = [0, 1, 2].map(|axis| i64::from(origin[axis]) - i64::from(pivot[axis]));
            let (corner, turned_size) = rotation.turn_box(offset, size);
            let mut low = [0_i32; 3];
            for axis in 0..3 {
                let value = i64::from(pivot[axis]) + corner[axis];
                low[axis] = i32::try_from(value).map_err(|_| GhostError::Layout(LayoutIssue::TooLarge))?;
                world_min[axis] = world_min[axis].min(value);
                world_max[axis] = world_max[axis].max(value + i64::from(turned_size[axis]) - 1);
            }
            pieces.push(GhostPiece { structure: Arc::clone(turned), world_min: low, remap });
        }
        let marker_block = palette.marker_id()?;
        let mut min = [0_i32; 3];
        let mut max = [0_i32; 3];
        for axis in 0..3 {
            min[axis] = i32::try_from(world_min[axis]).map_err(|_| GhostError::Layout(LayoutIssue::TooLarge))?;
            max[axis] = i32::try_from(world_max[axis]).map_err(|_| GhostError::Layout(LayoutIssue::TooLarge))?;
        }
        Ok(GhostBuild { pieces, palette, marker_block, world_min: min, world_max: max, revision })
    }

    // A copy of `structure` with every cell at its turned position, so that the copy's cell
    // (0, 0, 0) is the lowest corner of the turned box. The palette is not changed: the shim turns
    // the block states. Linear in the volume; the game runs it on `GhostTurnWorker`'s thread.
    pub fn turn(structure: &Structure, rotation: Rotation) -> Structure {
        let never = AtomicBool::new(false);
        Self::turn_unless(structure, rotation, &never).unwrap_or_else(|| structure.clone())
    }

    // Like `turn`, but gives up with None as soon as `stop` is set; the flag is read before each X
    // slice of the structure.
    pub fn turn_unless(structure: &Structure, rotation: Rotation, stop: &AtomicBool) -> Option<Structure> {
        let size = [structure.size.x, structure.size.y, structure.size.z];
        let turned_size = rotation.turn_size(size);
        let corner = rotation.turn_corner(size);
        let empty = Structure {
            size: StructureSize::new(turned_size[0], turned_size[1], turned_size[2]),
            world_origin: structure.world_origin,
            palette: Vec::new(),
            primary: BlockLayer::default(),
            secondary: SparseLayer::default(),
        };
        let moved = |x: u32, y: u32, z: u32| -> usize {
            let turned = rotation.turn_offset([i64::from(x), i64::from(y), i64::from(z)]);
            let local = [0, 1, 2].map(|axis| u32::try_from(turned[axis] - corner[axis]).unwrap_or(u32::MAX));
            empty.index_of(local[0], local[1], local[2])
        };
        let source = structure.primary.as_slice();
        let mut cells = vec![BlockLayer::NONE; source.len()];
        let mut next = 0_usize;
        for x in 0..size[0] {
            if stop.load(Ordering::SeqCst) {
                return None;
            }
            for y in 0..size[1] {
                for z in 0..size[2] {
                    if let (Some(index), Some(slot)) = (source.get(next), cells.get_mut(moved(x, y, z))) {
                        *slot = *index;
                    }
                    next += 1;
                }
            }
        }
        let (sy, sz) = (size[1].max(1), size[2].max(1));
        let (waterlog_cells, waterlog_blocks): (Vec<u32>, Vec<u16>) = structure
            .secondary
            .iter()
            .map(|(cell, index)| {
                let (x, y, z) = (cell / (sy * sz), (cell / sz) % sy, cell % sz);
                (u32::try_from(moved(x, y, z)).unwrap_or(u32::MAX), index)
            })
            .unzip();
        Some(Structure {
            palette: structure.palette.clone(),
            primary: BlockLayer::new(cells),
            secondary: SparseLayer::new(waterlog_cells, waterlog_blocks),
            ..empty
        })
    }

    // Gives each block state a cell uses a build palette id. Air, structure void and indices no
    // cell uses get `GhostPalette::NONE`.
    fn remap(
        structure: &Structure,
        palette: &mut GhostPalette,
        ids: &mut HashMap<BlockState, u16>,
    ) -> Result<Vec<u16>, GhostError> {
        let mut used = vec![false; structure.palette.len()];
        for index in structure.primary.as_slice() {
            if let Some(slot) = used.get_mut(usize::from(*index)) {
                *slot = true;
            }
        }
        let mut remap = Vec::with_capacity(structure.palette.len());
        for (block, used) in structure.palette.iter().zip(used) {
            if !used || block.is_empty_block() {
                remap.push(GhostPalette::NONE);
                continue;
            }
            if let Some(id) = ids.get(block) {
                remap.push(*id);
                continue;
            }
            if palette.entries.len() >= GhostPalette::MAX_ENTRIES {
                return Err(GhostError::PaletteTooLarge { entries: palette.entries.len() + 1 });
            }
            // The check above keeps the length below `GhostPalette::NONE`.
            let id = palette.entries.len() as u16;
            palette.entries.push(block.clone());
            ids.insert(block.clone(), id);
            remap.push(id);
        }
        Ok(remap)
    }
}
