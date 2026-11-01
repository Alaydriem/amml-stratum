use std::ffi::c_void;

use amml_game::BlockPos;
use stratum_core::structs::block::Face;
use stratum_core::structs::easy_place::{GameMode as UseGameMode, UseHit, UseProbe};

use crate::game::{GameMode, PlayerView, WorldBlocks};

/// Reads the player's view for one use call: the eye, the view direction, the cells the player's box fills, the reach,
/// the game mode and the hit. The hit is the first block on the view ray within reach that cannot be built over, so a
/// ghost spot behind grass is not reached through a wall; with no such block it is the game's own hit when that is a
/// block. Read-only, and it does not allocate. The time, the press flags and the swap answer are the caller's.
pub struct ProbeReader;

impl ProbeReader {
    // the most cell crossings one walk makes; a reach of 12 crosses at most 3 * 13 cells
    const MAX_CROSSINGS: usize = 64;
    // GameType: Survival 0, Creative 1
    const SURVIVAL: i32 = 0;
    const CREATIVE: i32 = 1;
    // HitResultType::Tile
    const TILE: i32 = 0;
    const MIN_LENGTH: f64 = 1e-9;
    // the face a ray enters a cell through, by the axis it stepped on and whether the step was positive: a step to +X
    // enters through the cell's West face
    const ENTERED_FACE: [[Face; 2]; 3] = [[Face::West, Face::East], [Face::Down, Face::Up], [Face::North, Face::South]];

    /// Fills the view fields of `probe` from the game.
    ///
    /// # Safety
    /// `region` must be the live `BlockSource` of the world the view's player is in.
    #[allow(unsafe_code)]
    pub unsafe fn read(
        view: &PlayerView,
        blocks: &WorldBlocks,
        region: *mut c_void,
        game_mode: &GameMode,
        probe: &mut UseProbe,
    ) {
        let eye = view.eye();
        let dir = view.view_vector(1.0);
        probe.eye = eye.map(f64::from);
        let dir = dir.map(f64::from);
        let length = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if length > Self::MIN_LENGTH {
            probe.dir = dir.map(|axis| axis / length);
        }
        Self::read_body(view, probe);
        probe.reach = f64::from(game_mode.max_pick_range());
        probe.game_mode = match view.game_type() {
            Some(Self::SURVIVAL) => UseGameMode::Survival,
            Some(Self::CREATIVE) => UseGameMode::Creative,
            _ => UseGameMode::Other,
        };
        probe.hit = None;
        // SAFETY: forwarded to the caller
        if length > Self::MIN_LENGTH && unsafe { Self::walk(blocks, region, probe) } {
            return;
        }
        // SAFETY: forwarded to the caller
        unsafe { Self::game_hit(view, blocks, region, probe) };
    }

    // the cells the player's box overlaps, both inclusive; a box face on a cell edge does not reach into the next cell
    fn read_body(view: &PlayerView, probe: &mut UseProbe) {
        let Some(body) = view.body() else {
            probe.body = None;
            return;
        };
        let mut low = [0i32; 3];
        let mut high = [0i32; 3];
        for axis in 0..3 {
            low[axis] = f64::from(body.min[axis]).floor() as i32;
            high[axis] = f64::from(body.max[axis]).ceil() as i32 - 1;
        }
        let ordered = (0..3).all(|axis| low[axis] <= high[axis]);
        probe.body = ordered.then_some((low, high));
    }

    // steps the ray from the eye cell to each cell it enters, nearest first, and stops at the first cell past the reach
    #[allow(unsafe_code)]
    unsafe fn walk(blocks: &WorldBlocks, region: *mut c_void, probe: &mut UseProbe) -> bool {
        let mut cell = [0i32; 3];
        let mut step = [0i32; 3];
        let mut next = [0f64; 3];
        let mut delta = [0f64; 3];
        for axis in 0..3 {
            let origin = probe.eye[axis];
            let d = probe.dir[axis];
            cell[axis] = origin.floor() as i32;
            if d == 0.0 {
                step[axis] = 0;
                delta[axis] = f64::INFINITY;
                next[axis] = f64::INFINITY;
                continue;
            }
            step[axis] = if d > 0.0 { 1 } else { -1 };
            delta[axis] = (1.0 / d).abs();
            let to_edge = if d > 0.0 { f64::from(cell[axis]) + 1.0 - origin } else { origin - f64::from(cell[axis]) };
            next[axis] = to_edge * delta[axis];
        }
        for _ in 0..Self::MAX_CROSSINGS {
            // ties step x, then y, then z, the same order as the core ghost walk
            let axis = if next[0] <= next[1] && next[0] <= next[2] {
                0
            } else if next[1] <= next[2] {
                1
            } else {
                2
            };
            let t = next[axis];
            if t > probe.reach {
                return false;
            }
            cell[axis] += step[axis];
            next[axis] += delta[axis];
            let pos = BlockPos { x: cell[0], y: cell[1], z: cell[2] };
            // SAFETY: forwarded to the caller
            if unsafe { blocks.replaceable(region, &pos) } {
                continue;
            }
            let face = Self::ENTERED_FACE[axis][if step[axis] > 0 { 0 } else { 1 }];
            probe.hit = Some(UseHit { cell, face, distance: t, replaceable: false });
            return true;
        }
        false
    }

    #[allow(unsafe_code)]
    unsafe fn game_hit(view: &PlayerView, blocks: &WorldBlocks, region: *mut c_void, probe: &mut UseProbe) {
        let Some(hit) = view.hit_result() else { return };
        // SAFETY: the level's live HitResult, which the mirror lays out up to mPos
        let hit = unsafe { &*hit };
        let Some(face) = Face::ALL.get(usize::from(hit.facing)).copied() else { return };
        if hit.kind != Self::TILE {
            return;
        }
        let dx = f64::from(hit.pos[0]) - probe.eye[0];
        let dy = f64::from(hit.pos[1]) - probe.eye[1];
        let dz = f64::from(hit.pos[2]) - probe.eye[2];
        // SAFETY: forwarded to the caller
        let replaceable = unsafe { blocks.replaceable(region, &hit.block) };
        probe.hit = Some(UseHit {
            cell: [hit.block.x, hit.block.y, hit.block.z],
            face,
            distance: (dx * dx + dy * dy + dz * dz).sqrt(),
            replaceable,
        });
    }
}
