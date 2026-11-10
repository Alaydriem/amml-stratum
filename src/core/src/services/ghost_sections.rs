use super::SectionScan;
use crate::structs::ghost::{
    GhostBuild, GhostDraw, GhostJob, GhostJobMode, GhostPalette, GhostView, SectionKey, SectionPart,
};

// Cuts a ghost build into section keys and fills the padded cell grids the mesh jobs read.
pub struct GhostSections;

impl GhostSections {
    // Blocks along the longest side of the mini ghost.
    pub const MINIATURE_SIZE: f32 = 2.0;

    // The largest scale of the mini ghost, so a small build is never drawn near full size.
    pub const MAX_FRACTION: f32 = 0.25;

    // The key of the structure block marker beside the build's lowest corner.
    pub const MARKER_KEY: SectionKey = SectionKey { x: 0, y: 0, z: 0, layer: None, part: SectionPart::Marker };

    // World blocks per mesh cell of the mini ghost: `MINIATURE_SIZE` over the build's longest side,
    // at most `MAX_FRACTION`.
    pub fn miniature_scale(build: &GhostBuild) -> f32 {
        let longest = (0..3)
            .map(|axis| i64::from(build.world_max[axis]) - i64::from(build.world_min[axis]) + 1)
            .max()
            .unwrap_or(1)
            .max(1);
        (Self::MINIATURE_SIZE / longest as f32).min(Self::MAX_FRACTION)
    }

    // Where the slot built for `key` is drawn. A full part sits at `built_origin`, the origin of the
    // slot's mesh. The marker and the mini ghost sit diagonally outside the build's lowest corner,
    // so the full ghost never covers them: the marker one block out on X and Z, and the mini ghost,
    // scaled about that corner, with its high X and Z edges on it. A mini ghost group is one box from the corner. The caller adds any move shift.
    pub fn draw(build: &GhostBuild, key: SectionKey, slot: u32, built_origin: [i32; 3]) -> GhostDraw {
        let min = build.world_min;
        if key.part == SectionPart::Marker {
            return GhostDraw { slot, origin: [min[0] - 1, min[1], min[2] - 1], offset: [0; 3], scale: 1.0 };
        }
        if key.part == SectionPart::MiniatureGroup {
            return GhostDraw {
                slot,
                origin: min,
                offset: [min[0] - build.world_max[0] - 1, 0, min[2] - build.world_max[2] - 1],
                scale: Self::miniature_scale(build),
            };
        }
        GhostDraw { slot, origin: built_origin, offset: [0; 3], scale: 1.0 }
    }

    // Every Whole-view section that holds a cell. A section with more cells than
    // `2 * cells_per_frame` becomes a `Lower` and an `Upper` key, each kept only when it holds a
    // cell, and `cells[i]` is the filled cell count of `out[i]`. This is the whole scan of a build in one call; the game thread uses `SectionScan`
    // instead and spreads it over ticks.
    //
    // Limit: a key can hold more cells than `cells_per_frame`. A Whole key holds up to
    // `2 * cells_per_frame` cells and a half up to 2,048, and the scheduler still hands out one job
    // per frame, so the frame that builds a dense key takes longer than the frame budget: at 2 to
    // 3 us per cell, about 3.5 ms for a Whole key at the default 600 and about 6 ms for a solid half.
    pub fn occupied(build: &GhostBuild, cells_per_frame: u32, out: &mut Vec<SectionKey>, cells: &mut Vec<u32>) {
        out.clear();
        cells.clear();
        let mut scan = SectionScan::new(build);
        while !scan.step(build, cells_per_frame, usize::MAX, out, cells) {}
    }

    // The keys to build for `view` out of the `occupied` keys, nearest to `feet` first. A section
    // column is wanted when its chunk is within `draw_distance_chunks` of the player's chunk,
    // counted as the larger of the two horizontal chunk offsets. A Layer(y) key is made for each
    // column with an occupied key whose Y range covers `y`, even when that layer row holds no cell.
    // Touches no cells.
    pub fn wanted(
        occupied: &[SectionKey],
        view: GhostView,
        feet: [i32; 3],
        draw_distance_chunks: u32,
        out: &mut Vec<SectionKey>,
    ) {
        out.clear();
        let reach = i32::try_from(draw_distance_chunks).unwrap_or(i32::MAX);
        let player_x = feet[0].div_euclid(SectionKey::SIZE);
        let player_z = feet[2].div_euclid(SectionKey::SIZE);
        for key in occupied {
            let near = key.x.abs_diff(player_x).max(key.z.abs_diff(player_z)) <= reach.unsigned_abs();
            if !near {
                continue;
            }
            match view {
                GhostView::Whole => out.push(*key),
                GhostView::Layer(y) => {
                    let local = y.rem_euclid(SectionKey::SIZE);
                    let covers = key.y == y.div_euclid(SectionKey::SIZE)
                        && local >= key.part.y_offset()
                        && local < key.part.y_offset() + key.part.height() as i32;
                    if covers {
                        out.push(SectionKey { layer: Some(y), part: SectionPart::Whole, ..*key });
                    }
                }
            }
        }

        out.sort_unstable_by(|a, b| {
            a.distance_sq(feet)
                .cmp(&b.distance_sq(feet))
                .then_with(|| (a.y, a.x, a.z, a.part).cmp(&(b.y, b.x, b.z, b.part)))
        });
        out.dedup();
    }

    // Refills `job` for `key` in place. The slot is left as it was.
    //
    // Each padded cell is read with `GhostBuild::block_at`, which scans the pieces, so a Whole job
    // costs up to 5832 lookups. Cells outside the build's box are skipped without a lookup.
    //
    // The marker part is one cell, `build.marker_block`.
    pub fn fill_job(build: &GhostBuild, key: SectionKey, job: &mut GhostJob) {
        job.mode = GhostJobMode::Replace;
        job.group_serial = 0;
        job.group_index = 0;
        job.group_offset = [0; 3];
        if key.part == SectionPart::Marker {
            job.key = key;
            job.skip_matching = false;
            job.origin = build.world_min;
            job.dims = [1, 1, 1];
            job.slot_size = job.dims;
            job.cells.clear();
            job.cells.resize(27, GhostPalette::NONE);
            job.cells[13] = build.marker_block;
            return;
        }
        job.skip_matching = true;
        let (origin_y, height) = match key.layer {
            Some(layer) => (layer, 1),
            None => (key.y * SectionKey::SIZE + key.part.y_offset(), key.part.height()),
        };
        let origin = [key.x * SectionKey::SIZE, origin_y, key.z * SectionKey::SIZE];
        let dims = [SectionKey::SIZE as u32, height, SectionKey::SIZE as u32];
        job.key = key;
        job.origin = origin;
        job.dims = dims;
        job.slot_size = dims;

        let padded = [dims[0] as usize + 2, dims[1] as usize + 2, dims[2] as usize + 2];
        job.cells.clear();
        job.cells.resize(padded[0] * padded[1] * padded[2], GhostPalette::NONE);

        let mut index = 0;
        for py in 0..padded[1] {
            let row_drawn = key.layer.is_none() || py == 1;
            let wy = origin[1] + py as i32 - 1;
            for pz in 0..padded[2] {
                let wz = origin[2] + pz as i32 - 1;
                for px in 0..padded[0] {
                    let wx = origin[0] + px as i32 - 1;
                    if row_drawn
                        && Self::inside(build, [wx, wy, wz])
                        && let Some(id) = build.block_at([wx, wy, wz])
                    {
                        job.cells[index] = id;
                    }
                    index += 1;
                }
            }
        }
    }

    // Refills `job` as an Append job of a group build: the cells of the full key `member`, placed
    // by `group_offset` inside a slot box the size of the build. The group serial and index are
    // the scheduler's to set.
    pub fn fill_member_job(build: &GhostBuild, member: SectionKey, job: &mut GhostJob) {
        Self::fill_job(build, member, job);
        job.mode = GhostJobMode::Append;
        job.skip_matching = false;
        job.group_offset = [0, 1, 2].map(|axis| job.origin[axis] - build.world_min[axis]);
        job.origin = build.world_min;
        job.slot_size = Self::build_size(build);
    }

    // Refills `job` as the Finish job of a group build for `key`: no cells.
    pub fn fill_finish_job(build: &GhostBuild, key: SectionKey, job: &mut GhostJob) {
        job.key = key;
        job.mode = GhostJobMode::Finish;
        job.group_serial = 0;
        job.group_index = 0;
        job.group_offset = [0; 3];
        job.skip_matching = false;
        job.origin = build.world_min;
        job.dims = [0; 3];
        job.slot_size = Self::build_size(build);
        job.cells.clear();
    }

    // The build's size in blocks.
    fn build_size(build: &GhostBuild) -> [u32; 3] {
        [0, 1, 2].map(|axis| (build.world_max[axis] - build.world_min[axis] + 1) as u32)
    }

    pub(crate) fn section_bounds(build: &GhostBuild) -> ([i32; 3], [i32; 3]) {
        (
            build.world_min.map(|v| v.div_euclid(SectionKey::SIZE)),
            build.world_max.map(|v| v.div_euclid(SectionKey::SIZE)),
        )
    }

    fn inside(build: &GhostBuild, world: [i32; 3]) -> bool {
        (0..3).all(|axis| world[axis] >= build.world_min[axis] && world[axis] <= build.world_max[axis])
    }

    // Pushes the Whole key of a section, or its halves when it holds more than
    // `2 * cells_per_frame` cells, and the filled cell count of each key to `cells`. Returns false, with no cell read, when no piece reaches into the
    // section.
    pub(crate) fn push_whole(
        build: &GhostBuild,
        section: [i32; 3],
        cells_per_frame: u32,
        out: &mut Vec<SectionKey>,
        cells: &mut Vec<u32>,
    ) -> bool {
        if !Self::touches_piece(build, section) {
            return false;
        }
        let base = section.map(|v| v * SectionKey::SIZE);
        let lower = Self::count(build, base, [16, 8, 16]);
        let upper = Self::count(build, [base[0], base[1] + 8, base[2]], [16, 8, 16]);
        let key = |part| SectionKey { x: section[0], y: section[1], z: section[2], layer: None, part };
        if lower + upper > cells_per_frame.saturating_mul(2) {
            if lower > 0 {
                out.push(key(SectionPart::Lower));
                cells.push(lower);
            }
            if upper > 0 {
                out.push(key(SectionPart::Upper));
                cells.push(upper);
            }
        } else if lower + upper > 0 {
            out.push(key(SectionPart::Whole));
            cells.push(lower + upper);
        }
        true
    }

    // True when some piece's box overlaps the section's box.
    fn touches_piece(build: &GhostBuild, section: [i32; 3]) -> bool {
        build.pieces.iter().any(|piece| {
            let size = [piece.structure.size.x, piece.structure.size.y, piece.structure.size.z];
            (0..3).all(|axis| {
                let low = i64::from(section[axis]) * i64::from(SectionKey::SIZE);
                let high = low + i64::from(SectionKey::SIZE) - 1;
                let piece_low = i64::from(piece.world_min[axis]);
                let piece_high = piece_low + i64::from(size[axis]) - 1;
                piece_low <= high && piece_high >= low
            })
        })
    }

    // Filled cells in the box of `dims` cells starting at `origin`.
    fn count(build: &GhostBuild, origin: [i32; 3], dims: [i32; 3]) -> u32 {
        let low = [0, 1, 2].map(|axis| origin[axis].max(build.world_min[axis]));
        let high = [0, 1, 2].map(|axis| (origin[axis] + dims[axis] - 1).min(build.world_max[axis]));
        let mut total = 0;
        for x in low[0]..=high[0] {
            for y in low[1]..=high[1] {
                for z in low[2]..=high[2] {
                    if build.block_at([x, y, z]).is_some() {
                        total += 1;
                    }
                }
            }
        }
        total
    }
}
