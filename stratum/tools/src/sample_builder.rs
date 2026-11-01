use stratum_core::errors::NbtWriteError;
use stratum_core::services::NbtWriter;
use stratum_core::structs::nbt::{NbtCompound, NbtTag};

// Writes synthetic `.mcstructure` samples in the form Minecraft writes them. The output is a
// fixed pattern, so every run gives the same bytes.
pub struct SampleBuilder;

impl SampleBuilder {
    const SIZE: [usize; 3] = [128, 64, 128];
    const STANDARD_SIZE: [usize; 3] = [64, 192, 64];
    const PIECE: usize = 64;
    const FLOOR_SPACING: usize = 8;
    const PANE_HEIGHT: usize = 6;
    const PALETTE_VERSION: i32 = 18_168_865;

    const NONE: i32 = -1;
    const STONE: i32 = 0;
    const PLANKS: i32 = 1;
    // Four stair directions follow, then the rest.
    const STAIRS: i32 = 2;
    const SLAB: i32 = 6;
    const FENCE: i32 = 7;
    const PANE: i32 = 8;
    const GRASS: i32 = 9;

    // The wood sets of the tint sample, one column each: log, wood, planks.
    const WOOD_SETS: [[&'static str; 3]; 12] = [
        ["minecraft:oak_log", "minecraft:oak_wood", "minecraft:oak_planks"],
        ["minecraft:spruce_log", "minecraft:spruce_wood", "minecraft:spruce_planks"],
        ["minecraft:birch_log", "minecraft:birch_wood", "minecraft:birch_planks"],
        ["minecraft:jungle_log", "minecraft:jungle_wood", "minecraft:jungle_planks"],
        ["minecraft:acacia_log", "minecraft:acacia_wood", "minecraft:acacia_planks"],
        ["minecraft:dark_oak_log", "minecraft:dark_oak_wood", "minecraft:dark_oak_planks"],
        ["minecraft:mangrove_log", "minecraft:mangrove_wood", "minecraft:mangrove_planks"],
        ["minecraft:cherry_log", "minecraft:cherry_wood", "minecraft:cherry_planks"],
        ["minecraft:pale_oak_log", "minecraft:pale_oak_wood", "minecraft:pale_oak_planks"],
        ["minecraft:crimson_stem", "minecraft:crimson_hyphae", "minecraft:crimson_planks"],
        ["minecraft:warped_stem", "minecraft:warped_hyphae", "minecraft:warped_planks"],
        ["minecraft:bamboo_block", "minecraft:stripped_bamboo_block", "minecraft:bamboo_planks"],
    ];
    // Every leaves block, one column each.
    const LEAVES: [&'static str; 11] = [
        "minecraft:oak_leaves",
        "minecraft:spruce_leaves",
        "minecraft:birch_leaves",
        "minecraft:jungle_leaves",
        "minecraft:acacia_leaves",
        "minecraft:dark_oak_leaves",
        "minecraft:mangrove_leaves",
        "minecraft:cherry_leaves",
        "minecraft:pale_oak_leaves",
        "minecraft:azalea_leaves",
        "minecraft:azalea_leaves_flowered",
    ];
    // The plants on the tint sample's grass row, with a bare grass block between each.
    const PLANTS: [&'static str; 3] = ["minecraft:short_grass", "minecraft:fern", "minecraft:stone"];
    const GRASS_BLOCK: &'static str = "minecraft:grass_block";
    // Cells between two columns of the tint sample, so every face of a block shows.
    const TINT_GAP: usize = 2;

    // The 128 x 64 x 128 build.
    pub fn large() -> Result<Vec<u8>, NbtWriteError> {
        let cells = Self::cells(Self::SIZE);
        Self::encode(Self::SIZE, [1000, 64, 2000], |x, y, z| cells[Self::index(Self::SIZE, x, y, z)])
    }

    // A 64 x 192 x 64 tower with the same block mix and fill as the large build.
    pub fn standard() -> Result<Vec<u8>, NbtWriteError> {
        let size = Self::STANDARD_SIZE;
        let cells = Self::cells(size);
        Self::encode(size, [1000, 64, 3000], |x, y, z| cells[Self::index(size, x, y, z)])
    }

    // The large build cut at x = 64 and z = 64. Index is `px * 2 + pz`.
    pub fn pieces() -> Result<[Vec<u8>; 4], NbtWriteError> {
        let cells = Self::cells(Self::SIZE);
        let mut pieces = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for (piece, bytes) in pieces.iter_mut().enumerate() {
            let (px, pz) = (piece / 2, piece % 2);
            let origin = [1000 + (px * Self::PIECE) as i32, 64, 2000 + (pz * Self::PIECE) as i32];
            let size = [Self::PIECE, Self::SIZE[1], Self::PIECE];
            *bytes = Self::encode(size, origin, |x, y, z| {
                cells[Self::index(Self::SIZE, px * Self::PIECE + x, y, pz * Self::PIECE + z)]
            })?;
        }
        Ok(pieces)
    }

    // Eight standard pieces in a 2 x 4 grid on x and z. Index is `px * 4 + pz`.
    pub fn target_pieces() -> Result<[Vec<u8>; 8], NbtWriteError> {
        let size = Self::STANDARD_SIZE;
        let cells = Self::cells(size);
        let mut pieces: [Vec<u8>; 8] = Default::default();
        for (piece, bytes) in pieces.iter_mut().enumerate() {
            let (px, pz) = (piece / 4, piece % 4);
            let origin = [1000 + (px * Self::PIECE) as i32, 64, 2000 + (pz * Self::PIECE) as i32];
            *bytes = Self::encode(size, origin, |x, y, z| cells[Self::index(size, x, y, z)])?;
        }
        Ok(pieces)
    }

    // Every wood set, every leaves block and the grass, for checking the biome tint. One block per
    // column on x with TINT_GAP cells between. Rows on z: logs, woods, planks, leaves, then a row of
    // grass blocks that carries each plant on every second block. Stone on that row is the control.
    pub fn tint() -> Result<Vec<u8>, NbtWriteError> {
        let mut names: Vec<&str> = Vec::new();
        let mut rows: Vec<Vec<i32>> = Vec::new();
        let index = |name: &'static str, names: &mut Vec<&str>| -> i32 {
            match names.iter().position(|known| *known == name) {
                Some(found) => found as i32,
                None => {
                    names.push(name);
                    names.len() as i32 - 1
                }
            }
        };
        for part in 0..3 {
            rows.push(Self::WOOD_SETS.iter().map(|set| index(set[part], &mut names)).collect());
        }
        rows.push(Self::LEAVES.iter().map(|name| index(name, &mut names)).collect());
        let grass = index(Self::GRASS_BLOCK, &mut names);
        let plants: Vec<i32> = Self::PLANTS.iter().map(|name| index(name, &mut names)).collect();
        let columns = rows.iter().map(Vec::len).max().unwrap_or(0).max(plants.len() * 2);
        let step = Self::TINT_GAP + 1;
        let size = [columns * step, 2, (rows.len() + 1) * step];
        let plant_row = rows.len();
        let cell = |x: usize, y: usize, z: usize| -> i32 {
            if !x.is_multiple_of(step) || !z.is_multiple_of(step) {
                return Self::NONE;
            }
            let (column, row) = (x / step, z / step);
            if row == plant_row {
                return match (y, column % 2) {
                    (0, _) => grass,
                    (1, 0) => plants.get(column / 2).copied().unwrap_or(Self::NONE),
                    _ => Self::NONE,
                };
            }
            match y {
                0 => rows[row].get(column).copied().unwrap_or(Self::NONE),
                _ => Self::NONE,
            }
        };
        Self::encode_with(size, [1000, 64, 4000], Self::plain_palette(&names), cell)
    }

    // Every sample with the file name it is written under: `large.mcstructure`,
    // `standard.mcstructure`, `tint.mcstructure`, `piece-<px>-<pz>.mcstructure`, then `target-piece-<i>.mcstructure`.
    pub fn files() -> Result<Vec<(String, Vec<u8>)>, NbtWriteError> {
        let mut files = vec![
            ("large.mcstructure".to_string(), Self::large()?),
            ("standard.mcstructure".to_string(), Self::standard()?),
            ("tint.mcstructure".to_string(), Self::tint()?),
        ];
        for (index, bytes) in Self::pieces()?.into_iter().enumerate() {
            files.push((format!("piece-{}-{}.mcstructure", index / 2, index % 2), bytes));
        }
        for (index, bytes) in Self::target_pieces()?.into_iter().enumerate() {
            files.push((format!("target-piece-{index}.mcstructure"), bytes));
        }
        Ok(files)
    }

    // X slowest, then Y, then Z.
    fn index(size: [usize; 3], x: usize, y: usize, z: usize) -> usize {
        (x * size[1] + y) * size[2] + z
    }

    fn cells(size: [usize; 3]) -> Vec<i32> {
        let [sx, sy, sz] = size;
        let mut cells = vec![Self::NONE; sx * sy * sz];
        for x in 0..sx {
            for y in 0..sy {
                for z in 0..sz {
                    cells[Self::index(size, x, y, z)] = Self::block(size, x, y, z);
                }
            }
        }
        cells
    }

    fn block(size: [usize; 3], x: usize, y: usize, z: usize) -> i32 {
        let [sx, _, sz] = size;
        if y == 0 || x == 0 || z == 0 || x == sx - 1 || z == sz - 1 {
            return Self::STONE;
        }
        // Rows above the nearest floor at or below this level; the ground counts as floor 0.
        let above = y % Self::FLOOR_SPACING;
        if above == 0 {
            return Self::PLANKS;
        }
        // Glass pane partitions with a gap every fourth cell.
        if x % 16 == 8 && !z.is_multiple_of(4) && above <= Self::PANE_HEIGHT {
            return Self::PANE;
        }
        if above != 1 {
            return Self::NONE;
        }
        match (x * 7 + z * 13) % 8 {
            0 | 4 => Self::STAIRS + ((x + z) % 4) as i32,
            1 => Self::SLAB,
            2 => Self::FENCE,
            3 | 5 => Self::GRASS,
            _ => Self::NONE,
        }
    }

    fn palette() -> NbtTag {
        let stairs =
            |direction: i32| vec![("weirdo_direction", NbtTag::Int(direction)), ("upside_down_bit", NbtTag::Byte(0))];
        let mut entries: Vec<(&str, Vec<(&str, NbtTag)>)> =
            vec![("minecraft:stone", vec![]), ("minecraft:oak_planks", vec![])];
        for direction in 0..4 {
            entries.push(("minecraft:oak_stairs", stairs(direction)));
        }
        entries.push(("minecraft:oak_slab", vec![("minecraft:vertical_half", NbtTag::String("bottom".to_string()))]));
        entries.push(("minecraft:oak_fence", vec![]));
        entries.push(("minecraft:glass_pane", vec![]));
        entries.push(("minecraft:short_grass", vec![]));
        Self::palette_of(entries)
    }

    // Default-state palette entries for `names`, in order.
    fn plain_palette(names: &[&str]) -> NbtTag {
        Self::palette_of(names.iter().map(|name| (*name, Vec::new())).collect())
    }

    fn palette_of(entries: Vec<(&str, Vec<(&str, NbtTag)>)>) -> NbtTag {
        NbtTag::List(
            entries
                .into_iter()
                .map(|(name, states)| {
                    let states = states.into_iter().map(|(key, tag)| (key.to_string(), tag)).collect();
                    NbtTag::Compound(NbtCompound::new(vec![
                        ("name".to_string(), NbtTag::String(name.to_string())),
                        ("states".to_string(), NbtTag::Compound(NbtCompound::new(states))),
                        ("version".to_string(), NbtTag::Int(Self::PALETTE_VERSION)),
                    ]))
                })
                .collect(),
        )
    }

    fn encode(
        size: [usize; 3],
        origin: [i32; 3],
        cell: impl Fn(usize, usize, usize) -> i32,
    ) -> Result<Vec<u8>, NbtWriteError> {
        Self::encode_with(size, origin, Self::palette(), cell)
    }

    fn encode_with(
        size: [usize; 3],
        origin: [i32; 3],
        palette: NbtTag,
        cell: impl Fn(usize, usize, usize) -> i32,
    ) -> Result<Vec<u8>, NbtWriteError> {
        let mut primary = Vec::with_capacity(size[0] * size[1] * size[2]);
        for x in 0..size[0] {
            for y in 0..size[1] {
                for z in 0..size[2] {
                    primary.push(cell(x, y, z));
                }
            }
        }
        let secondary = vec![Self::NONE; primary.len()];
        let default = NbtCompound::new(vec![
            ("block_palette".to_string(), palette),
            ("block_position_data".to_string(), NbtTag::Compound(NbtCompound::default())),
        ]);
        let palette = NbtCompound::new(vec![("default".to_string(), NbtTag::Compound(default))]);
        let structure = NbtCompound::new(vec![
            ("block_indices".to_string(), NbtTag::List(vec![NbtTag::IntList(primary), NbtTag::IntList(secondary)])),
            ("entities".to_string(), NbtTag::List(Vec::new())),
            ("palette".to_string(), NbtTag::Compound(palette)),
        ]);
        NbtWriter::compound(
            "",
            &[
                ("format_version", NbtTag::Int(1)),
                ("size", NbtTag::IntList(size.map(|axis| axis as i32).to_vec())),
                ("structure", NbtTag::Compound(structure)),
                ("structure_world_origin", NbtTag::IntList(origin.to_vec())),
            ],
        )
    }
}
