use amml_api::{BlockPos, BlockSource, Face as ApiFace, GameMode, GameType, HitResult, LocalPlayer};
use stratum_core::structs::block::Face;
use stratum_core::structs::easy_place::{GameMode as UseGameMode, UseHit, UseProbe};

/// Reads the player's view for one use call: the head, the view direction, the cells the player's box fills, the
/// reach, the game mode and the hit. The hit is the first block on the view ray within reach that cannot be built
/// over, so a ghost spot behind grass is not reached through a wall; with no such block it is the game's own hit when
/// that is a block. Read-only, and it does not allocate. The time, the press flags and the swap answer are the
/// caller's.
pub struct ProbeReader;

impl ProbeReader {
    // the most cell crossings one walk makes; a reach of 12 crosses at most 3 * 13 cells
    const MAX_CROSSINGS: usize = 64;
    const MIN_LENGTH: f64 = 1e-9;
    // the face a ray enters a cell through, by the axis it stepped on and whether the step was positive: a step to +X
    // enters through the cell's West face
    const ENTERED_FACE: [[Face; 2]; 3] = [[Face::West, Face::East], [Face::Down, Face::Up], [Face::North, Face::South]];

    /// Fills the view fields of `probe` from the loader's reads of the player.
    pub fn read(player: &LocalPlayer<'_>, source: &BlockSource<'_>, game_mode: &GameMode<'_>, probe: &mut UseProbe) {
        let head = player.head_position().unwrap_or_default();
        let dir = player.view_vector().unwrap_or_default();
        probe.eye = [f64::from(head.x), f64::from(head.y), f64::from(head.z)];
        let dir = [f64::from(dir.x), f64::from(dir.y), f64::from(dir.z)];
        let length = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        probe.dir = [0.0; 3];
        if length > Self::MIN_LENGTH {
            probe.dir = dir.map(|axis| axis / length);
        }
        probe.body = player
            .aabb()
            .and_then(|aabb| Self::body([aabb.min.x, aabb.min.y, aabb.min.z], [aabb.max.x, aabb.max.y, aabb.max.z]));
        probe.reach = f64::from(game_mode.max_pick_range().unwrap_or(0.0));
        probe.game_mode = match player.game_type() {
            Some(GameType::Survival) => UseGameMode::Survival,
            Some(GameType::Creative) => UseGameMode::Creative,
            _ => UseGameMode::Other,
        };
        probe.hit = None;
        if length > Self::MIN_LENGTH && Self::walk(|pos| Self::replaceable(source, pos), probe) {
            return;
        }
        Self::game_hit(player, source, probe);
    }

    // the game's own hit, when it is a block: its cell, face and distance from the eye, and whether the block there can
    // be built over
    fn game_hit(player: &LocalPlayer<'_>, source: &BlockSource<'_>, probe: &mut UseProbe) {
        let Some(HitResult::Block(hit)) = player.hit_result() else { return };
        let face = match hit.face {
            ApiFace::Down => Face::Down,
            ApiFace::Up => Face::Up,
            ApiFace::North => Face::North,
            ApiFace::South => Face::South,
            ApiFace::West => Face::West,
            ApiFace::East => Face::East,
            ApiFace::Other(_) => return,
        };
        let point = [f64::from(hit.point.x), f64::from(hit.point.y), f64::from(hit.point.z)];
        let distance = (0..3).map(|axis| (point[axis] - probe.eye[axis]).powi(2)).sum::<f64>().sqrt();
        probe.hit = Some(UseHit {
            cell: [hit.pos.x, hit.pos.y, hit.pos.z],
            face,
            distance,
            replaceable: Self::replaceable(source, hit.pos),
        });
    }

    /// The cells a box from `min` to `max` overlaps, both inclusive; a box face on a cell edge does not reach into the
    /// next cell. None for a box with no cell.
    pub fn body(min: [f32; 3], max: [f32; 3]) -> Option<([i32; 3], [i32; 3])> {
        let low: [i32; 3] = std::array::from_fn(|axis| f64::from(min[axis]).floor() as i32);
        let high: [i32; 3] = std::array::from_fn(|axis| f64::from(max[axis]).ceil() as i32 - 1);
        (0..3).all(|axis| low[axis] <= high[axis]).then_some((low, high))
    }

    /// Steps the ray of `probe` from the eye cell to each cell it enters, nearest first, and sets the hit at the first
    /// cell that is not `replaceable`; stops at the first cell past the reach. True when it hit.
    pub fn walk(mut replaceable: impl FnMut(BlockPos) -> bool, probe: &mut UseProbe) -> bool {
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
            if replaceable(BlockPos { x: cell[0], y: cell[1], z: cell[2] }) {
                continue;
            }
            let face = Self::ENTERED_FACE[axis][if step[axis] > 0 { 0 } else { 1 }];
            probe.hit = Some(UseHit { cell, face, distance: t, replaceable: false });
            return true;
        }
        false
    }

    // a block the loader cannot read is walked through, as air is
    fn replaceable(source: &BlockSource<'_>, pos: BlockPos) -> bool {
        source.block(pos).and_then(|block| block.can_be_built_over()).unwrap_or(true)
    }
}
