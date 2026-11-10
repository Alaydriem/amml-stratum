use std::collections::HashMap;
use std::sync::Arc;

use super::{ShapeRules, TexturePack};
use crate::errors::PreviewError;
use crate::structs::block::Face;
use crate::structs::preview::{
    BlockLook, BlockShape, BlockTexture, Occlusion, PreviewMesh, PreviewQuad, PreviewShade, PreviewStop,
};
use crate::structs::structure::{BlockLayer, StateValue, Structure};

// Builds the quads of a structure's visible faces. A face against a neighbour that hides it is left out. A mesh
// holds at most `max_quads` quads; the stop is checked once per row of cells.
pub struct PreviewMesher {
    max_quads: usize,
    stop: PreviewStop,
}

impl PreviewMesher {
    // The longest side the mesh can place: corners are i16 sixteenths.
    pub const MAX_SIDE: u32 = 2047;
    const MISSING: [u8; 4] = [160, 160, 160, 255];
    // The light of a cross plant, which the game draws a little darker than a top face.
    const CROSS_SHADE: u8 = 230;
    // The most quads one cell gives: a stairs block is two boxes of six faces.
    const CELL_QUADS: usize = 12;

    pub fn new(max_quads: usize, stop: PreviewStop) -> Self {
        Self { max_quads, stop }
    }

    // The visible faces of `structure`; `Stopped` when the stop was set during the build.
    pub fn build(&self, structure: &Structure, pack: &mut TexturePack) -> Result<PreviewMesh, PreviewError> {
        let size = structure.size;
        if size.x > Self::MAX_SIDE || size.y > Self::MAX_SIDE || size.z > Self::MAX_SIDE {
            return Err(PreviewError::TooLarge);
        }
        let missing = vec![Arc::new(BlockTexture::solid(Self::MISSING))];
        let mut mesh = PreviewMesh::new(size, missing, self.max_quads);
        let looks = Self::looks(structure, pack, &mut mesh);
        let occlusion: Vec<Occlusion> = looks.iter().map(|look| look.occlusion).collect();
        let cells = structure.primary.as_slice();
        let (sx, sy, sz) = (size.x as usize, size.y as usize, size.z as usize);
        let neighbour = |x: usize, y: usize, z: usize, face: Face| -> Occlusion {
            let [dx, dy, dz] = face.normal();
            let (nx, ny, nz) = (x as i64 + i64::from(dx), y as i64 + i64::from(dy), z as i64 + i64::from(dz));
            if nx < 0 || ny < 0 || nz < 0 || nx >= sx as i64 || ny >= sy as i64 || nz >= sz as i64 {
                return Occlusion::None;
            }
            let index = (nx as usize * sy + ny as usize) * sz + nz as usize;
            match cells.get(index) {
                Some(&palette) if palette != BlockLayer::NONE => {
                    occlusion.get(usize::from(palette)).copied().unwrap_or(Occlusion::None)
                }
                _ => Occlusion::None,
            }
        };
        for x in 0..sx {
            for y in 0..sy {
                if self.stop.is_stopped() {
                    return Err(PreviewError::Stopped);
                }
                for z in 0..sz {
                    let Some(&palette) = cells.get((x * sy + y) * sz + z) else {
                        continue;
                    };
                    let Some(look) = looks.get(usize::from(palette)) else {
                        continue;
                    };
                    if !mesh.has_room(Self::CELL_QUADS) {
                        mesh.truncated = true;
                        return Ok(mesh);
                    }
                    let cell = [x as i16 * 16, y as i16 * 16, z as i16 * 16];
                    let hidden = |face: Face| look.occlusion.hidden_by(neighbour(x, y, z, face));
                    match look.shape {
                        BlockShape::Hidden => {}
                        BlockShape::Cube => {
                            Self::emit_box(&mut mesh, cell, [0, 0, 0], [16, 16, 16], look, &hidden, None)
                        }
                        BlockShape::Box { min, max } => Self::emit_box(&mut mesh, cell, min, max, look, &hidden, None),
                        BlockShape::Stairs { upside_down, high_side } => {
                            Self::emit_stairs(&mut mesh, cell, upside_down, high_side, look, &hidden)
                        }
                        BlockShape::Cross => Self::emit_cross(&mut mesh, cell, look),
                    }
                }
            }
        }
        Ok(mesh)
    }

    // One look per palette entry, with its textures added to the mesh's table.
    fn looks(structure: &Structure, pack: &mut TexturePack, mesh: &mut PreviewMesh) -> Vec<BlockLook> {
        let mut table: HashMap<String, u16> = HashMap::new();
        let mut looks = Vec::with_capacity(structure.palette.len());
        for block in &structure.palette {
            let (shape, occlusion) = ShapeRules::shape(block);
            if shape == BlockShape::Hidden {
                looks.push(BlockLook::hidden());
                continue;
            }
            let Some(mut keys) = pack.face_keys(&block.name) else {
                mesh.missing += 1;
                looks.push(BlockLook::new(shape, [0; 6], occlusion));
                continue;
            };
            // a log lying along X or Z shows its rings on the ends instead of the top and bottom
            match block.states.get("pillar_axis") {
                Some(StateValue::Text(axis)) if axis == "x" => {
                    keys.swap(Face::Up.index(), Face::East.index());
                    keys.swap(Face::Down.index(), Face::West.index());
                }
                Some(StateValue::Text(axis)) if axis == "z" => {
                    keys.swap(Face::Up.index(), Face::North.index());
                    keys.swap(Face::Down.index(), Face::South.index());
                }
                _ => {}
            }
            // the upper half of a two block plant uses the top texture, the lower half the bottom one
            if shape == BlockShape::Cross {
                let upper =
                    matches!(block.states.get("upper_block_bit"), Some(StateValue::Bool(true) | StateValue::Int(1)));
                let has_halves = block.states.get("upper_block_bit").is_some();
                let face = if has_halves && !upper { Face::Down } else { Face::Up };
                let key = keys[face.index()].clone();
                keys = std::array::from_fn(|_| key.clone());
            }
            let mut textures = [0u16; 6];
            let mut found_any = false;
            for (slot, key) in textures.iter_mut().zip(keys.iter()) {
                if let Some(&index) = table.get(key) {
                    *slot = index;
                    found_any |= index != 0;
                    continue;
                }
                let index = match pack.texture(key) {
                    Some(texture) if mesh.textures.len() < usize::from(u16::MAX) => {
                        mesh.textures.push(texture);
                        (mesh.textures.len() - 1) as u16
                    }
                    _ => 0,
                };
                table.insert(key.clone(), index);
                found_any |= index != 0;
                *slot = index;
            }
            if !found_any {
                mesh.missing += 1;
            }
            looks.push(BlockLook::new(shape, textures, occlusion));
        }
        looks
    }

    fn emit_stairs(
        mesh: &mut PreviewMesh,
        cell: [i16; 3],
        upside_down: bool,
        high_side: Face,
        look: &BlockLook,
        hidden: &impl Fn(Face) -> bool,
    ) {
        let (slab_min, slab_max, step_y, inner) =
            if upside_down { (8, 16, (0, 8), Face::Up) } else { (0, 8, (8, 16), Face::Down) };
        Self::emit_box(mesh, cell, [0, slab_min, 0], [16, slab_max, 16], look, hidden, None);
        let (min, max) = match high_side {
            Face::West => ([0, step_y.0, 0], [8, step_y.1, 16]),
            Face::South => ([0, step_y.0, 8], [16, step_y.1, 16]),
            Face::North => ([0, step_y.0, 0], [16, step_y.1, 8]),
            _ => ([8, step_y.0, 0], [16, step_y.1, 16]),
        };
        Self::emit_box(mesh, cell, min, max, look, hidden, Some(inner));
    }

    // The faces of one box; a face on the cell's side that the neighbour hides, and the `skip` face, are left out.
    fn emit_box(
        mesh: &mut PreviewMesh,
        cell: [i16; 3],
        min: [u8; 3],
        max: [u8; 3],
        look: &BlockLook,
        hidden: &impl Fn(Face) -> bool,
        skip: Option<Face>,
    ) {
        let [x0, y0, z0] = min.map(i16::from);
        let [x1, y1, z1] = max.map(i16::from);
        for face in Face::ALL {
            if skip == Some(face) {
                continue;
            }
            let on_side = match face {
                Face::Up => y1 == 16,
                Face::Down => y0 == 0,
                Face::North => z0 == 0,
                Face::South => z1 == 16,
                Face::West => x0 == 0,
                Face::East => x1 == 16,
            };
            if on_side && hidden(face) {
                continue;
            }
            // local corners, then the texture position of each from where it sits in the cell
            let local: [[i16; 3]; 4] = match face {
                Face::Up => [[x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]],
                Face::Down => [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                Face::North => [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]],
                Face::South => [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                Face::West => [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                Face::East => [[x1, y0, z0], [x1, y0, z1], [x1, y1, z1], [x1, y1, z0]],
            };
            let uvs = local.map(|[x, y, z]| {
                let (u, v) = match face {
                    Face::Up => (x, z),
                    Face::Down => (x, 16 - z),
                    Face::North => (16 - x, 16 - y),
                    Face::South => (x, 16 - y),
                    Face::West => (z, 16 - y),
                    Face::East => (16 - z, 16 - y),
                };
                [u as u8, v as u8]
            });
            let [nx, ny, nz] = face.normal();
            mesh.push(PreviewQuad {
                corners: local.map(|[x, y, z]| [cell[0] + x, cell[1] + y, cell[2] + z]),
                uvs,
                texture: look.textures[face.index()],
                shade: PreviewShade::of(face),
                normal: [nx as i8, ny as i8, nz as i8],
            });
        }
    }

    fn emit_cross(mesh: &mut PreviewMesh, cell: [i16; 3], look: &BlockLook) {
        let uvs = [[0, 16], [16, 16], [16, 0], [0, 0]];
        let planes: [[[i16; 3]; 4]; 2] =
            [[[0, 0, 0], [16, 0, 16], [16, 16, 16], [0, 16, 0]], [[16, 0, 0], [0, 0, 16], [0, 16, 16], [16, 16, 0]]];
        for plane in planes {
            mesh.push(PreviewQuad {
                corners: plane.map(|[x, y, z]| [cell[0] + x, cell[1] + y, cell[2] + z]),
                uvs,
                texture: look.textures[Face::Up.index()],
                shade: Self::CROSS_SHADE,
                normal: [0, 0, 0],
            });
        }
    }
}
