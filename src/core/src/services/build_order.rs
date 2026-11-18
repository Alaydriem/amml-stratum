use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::services::BlockClassifier;
use crate::structs::build::{BlockClass, BuildPass, BuildStep, SnakeLayer};
use crate::structs::ghost::GhostPiece;

// Orders the blocks of a structure for a build, layer by layer.
//
// Inside a layer the rows run along the longer horizontal side and snake: each row goes the
// other way from the one before. A block that touches the last one is placed next, so a gap goes
// to a touching block before the build jumps to a part that does not touch.
pub struct BuildOrder;

impl BuildOrder {
    const NONE: u8 = 0;
    const PLAIN: u8 = 1;
    const ATTACHED: u8 = 2;
    const FLUID: u8 = 3;
    const OVER_FLUID: u8 = 4;
    const HANGING: u8 = 5;

    // The Main steps of one layer, in order. A palette entry at or past `skip.len()` is not skipped.
    pub fn layer(piece: &GhostPiece, y: u32, skip: &[bool]) -> Vec<BuildStep> {
        Self::plan(piece, y, skip, [Self::PLAIN, Self::ATTACHED], BuildPass::Main)
    }

    // The Hanging steps of one layer: the blocks that hang from the block above. They run after every Main layer,
    // so the block above is already in place.
    pub fn hanging_layer(piece: &GhostPiece, y: u32, skip: &[bool]) -> Vec<BuildStep> {
        Self::plan(piece, y, skip, [Self::HANGING, Self::NONE], BuildPass::Hanging)
    }

    // The Fluid steps of one layer: the fluids, then the gravity blocks that rest on a fluid.
    pub fn fluid_layer(piece: &GhostPiece, y: u32, skip: &[bool]) -> Vec<BuildStep> {
        Self::plan(piece, y, skip, [Self::FLUID, Self::OVER_FLUID], BuildPass::Fluid)
    }

    // The steps of both passes: every block that is not empty and not skipped.
    pub fn count(piece: &GhostPiece, skip: &[bool]) -> u64 {
        let structure = &piece.structure;
        let wanted: Vec<bool> = structure
            .palette
            .iter()
            .enumerate()
            .map(|(index, block)| !block.is_empty_block() && !Self::skipped(skip, index))
            .collect();
        structure
            .primary
            .as_slice()
            .iter()
            .filter(|cell| wanted.get(usize::from(**cell)).copied().unwrap_or(false))
            .count() as u64
    }

    fn skipped(skip: &[bool], palette: usize) -> bool {
        skip.get(palette).copied().unwrap_or(false)
    }

    fn plan(piece: &GhostPiece, y: u32, skip: &[bool], groups: [u8; 2], pass: BuildPass) -> Vec<BuildStep> {
        let size = piece.structure.size;
        if y >= size.y || size.x == 0 || size.z == 0 {
            return Vec::new();
        }
        let snake = if size.x >= size.z {
            SnakeLayer { long_is_x: true, long_len: size.x, short_len: size.z }
        } else {
            SnakeLayer { long_is_x: false, long_len: size.z, short_len: size.x }
        };
        let kinds = Self::kinds(piece, y, skip, snake);

        let mut steps = Vec::new();
        for group in groups {
            if group == Self::NONE {
                continue;
            }
            Self::walk(piece, y, &kinds, group, snake, pass, &mut steps);
        }
        steps
    }

    // The group of each cell of the layer, in snake order.
    fn kinds(piece: &GhostPiece, y: u32, skip: &[bool], snake: SnakeLayer) -> Vec<u8> {
        let structure = &piece.structure;
        let classes: Vec<Option<BlockClass>> = structure
            .palette
            .iter()
            .map(|block| (!block.is_empty_block()).then(|| BlockClassifier::class_of(block)))
            .collect();
        let class_at = |x: u32, y: u32, z: u32| -> Option<BlockClass> {
            let palette = structure.primary.get(structure.index_of(x, y, z))?;
            classes.get(usize::from(palette)).copied().flatten()
        };

        let mut kinds = vec![Self::NONE; snake.long_len as usize * snake.short_len as usize];
        for (index, kind) in kinds.iter_mut().enumerate() {
            let [x, z] = snake.cell_of(index as u32);
            let Some(palette) = structure.primary.get(structure.index_of(x, y, z)) else {
                continue;
            };
            if Self::skipped(skip, usize::from(palette)) {
                continue;
            }
            let Some(class) = classes.get(usize::from(palette)).copied().flatten() else {
                continue;
            };
            *kind = match class {
                BlockClass::Plain => Self::PLAIN,
                BlockClass::Attached => Self::ATTACHED,
                BlockClass::Hanging => Self::HANGING,
                BlockClass::Fluid => Self::FLUID,
                BlockClass::Gravity => {
                    let over_fluid = y > 0 && class_at(x, y - 1, z) == Some(BlockClass::Fluid);
                    if over_fluid { Self::OVER_FLUID } else { Self::PLAIN }
                }
            };
        }
        kinds
    }

    // Emits every cell of one group: the frontier first, and when it is empty the first cell left.
    fn walk(
        piece: &GhostPiece,
        y: u32,
        kinds: &[u8],
        group: u8,
        snake: SnakeLayer,
        pass: BuildPass,
        steps: &mut Vec<BuildStep>,
    ) {
        let structure = &piece.structure;
        let mut frontier: BinaryHeap<Reverse<u32>> = BinaryHeap::new();
        let mut queued = vec![false; kinds.len()];
        let mut placed = vec![false; kinds.len()];
        let mut scan = 0_usize;
        loop {
            let mut next = None;
            while let Some(Reverse(index)) = frontier.pop() {
                if !placed[index as usize] {
                    next = Some(index);
                    break;
                }
            }
            if next.is_none() {
                while scan < kinds.len() && (kinds[scan] != group || placed[scan]) {
                    scan += 1;
                }
                if scan < kinds.len() {
                    next = Some(scan as u32);
                }
            }
            let Some(index) = next else {
                return;
            };
            placed[index as usize] = true;

            let [x, z] = snake.cell_of(index);
            let palette = structure.primary.get(structure.index_of(x, y, z)).unwrap_or(0);
            steps.push(BuildStep {
                position: [
                    piece.world_min[0].saturating_add(x as i32),
                    piece.world_min[1].saturating_add(y as i32),
                    piece.world_min[2].saturating_add(z as i32),
                ],
                palette,
                pass,
            });

            let row = index / snake.long_len;
            let along = snake.along(index);
            let mut neighbours = [None; 4];
            if along > 0 {
                neighbours[0] = Some(snake.index_at(row, along - 1));
            }
            if along + 1 < snake.long_len {
                neighbours[1] = Some(snake.index_at(row, along + 1));
            }
            if row > 0 {
                neighbours[2] = Some(snake.index_at(row - 1, along));
            }
            if row + 1 < snake.short_len {
                neighbours[3] = Some(snake.index_at(row + 1, along));
            }
            for neighbour in neighbours.into_iter().flatten() {
                let slot = neighbour as usize;
                if kinds[slot] == group && !placed[slot] && !queued[slot] {
                    queued[slot] = true;
                    frontier.push(Reverse(neighbour));
                }
            }
        }
    }
}
