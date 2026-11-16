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
    const LEAVES: [&'static str; 14] = [
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
        "minecraft:orange_poplar_leaves",
        "minecraft:red_poplar_leaves",
        "minecraft:yellow_poplar_leaves",
    ];
    // The plants on the tint sample's grass row, with a bare grass block between each.
    const PLANTS: [&'static str; 3] = ["minecraft:short_grass", "minecraft:fern", "minecraft:stone"];
    const GRASS_BLOCK: &'static str = "minecraft:grass_block";
    // Cells between two columns of the tint sample, so every face of a block shows.
    const TINT_GAP: usize = 2;
    // The rows of the render check sample on z, one suspect group each. Stone heads every row as the control.
    const RENDER_CHECK: [&'static [&'static str]; 7] = [
        // translucent, drawn in the blend layer
        &[
            "minecraft:honey_block",
            "minecraft:slime",
            "minecraft:glass",
            "minecraft:white_stained_glass",
            "minecraft:tinted_glass",
            "minecraft:ice",
            "minecraft:packed_ice",
            "minecraft:honeycomb_block",
        ],
        // drawn by a block entity renderer, not the block tessellator
        &[
            "minecraft:chest",
            "minecraft:ender_chest",
            "minecraft:undyed_shulker_box",
            "minecraft:bed",
            "minecraft:standing_banner",
            "minecraft:standing_sign",
            "minecraft:skeleton_skull",
            "minecraft:bell",
            "minecraft:conduit",
            "minecraft:decorated_pot",
            "minecraft:enchanting_table",
            "minecraft:lectern",
        ],
        // blocks added in recent versions, likely built by the newer tessellation pipeline
        &[
            "minecraft:crafter",
            "minecraft:copper_bulb",
            "minecraft:trial_spawner",
            "minecraft:vault",
            "minecraft:heavy_core",
            "minecraft:chiseled_bookshelf",
            "minecraft:creaking_heart",
            "minecraft:resin_block",
            "minecraft:sculk_sensor",
            "minecraft:calibrated_sculk_sensor",
            "minecraft:pointed_dripstone",
            "minecraft:amethyst_cluster",
        ],
        // odd shapes and alpha-tested textures
        &[
            "minecraft:scaffolding",
            "minecraft:web",
            "minecraft:lantern",
            "minecraft:chain",
            "minecraft:end_rod",
            "minecraft:lightning_rod",
            "minecraft:azalea",
            "minecraft:spore_blossom",
            "minecraft:bamboo",
            "minecraft:kelp",
            "minecraft:campfire",
            "minecraft:brewing_stand",
        ],
        // liquids, portals and special render layers
        &[
            "minecraft:water",
            "minecraft:lava",
            "minecraft:powder_snow",
            "minecraft:portal",
            "minecraft:end_portal_frame",
            "minecraft:mob_spawner",
            "minecraft:beacon",
            "minecraft:cauldron",
            "minecraft:hopper",
            "minecraft:anvil",
            "minecraft:grindstone",
            "minecraft:stonecutter_block",
        ],
        // full opaque cubes that should always draw, then a rail, whose block type overrides the render layer by
        // position
        &[
            "minecraft:oak_planks",
            "minecraft:copper_block",
            "minecraft:diamond_block",
            "minecraft:obsidian",
            "minecraft:sea_lantern",
            "minecraft:glowstone",
            "minecraft:rail",
        ],
        // every leaves block: each has a biome or fixed tint, and some draw on a Seasons layer
        &Self::LEAVES,
    ];
    const CONTROL: &'static str = "minecraft:stone";

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
        Self::encode_with(size, [1000, 64, 4000], Self::plain_palette(&names), NbtCompound::default(), cell)
    }

    // One block of each suspect group per column on x, with TINT_GAP cells between, for finding the blocks the
    // in-world ghost does not draw. Each row on z is one group of RENDER_CHECK, and column 0 of every row is stone.
    // One layer high, every block in its default states.
    pub fn render_check() -> Result<Vec<u8>, NbtWriteError> {
        let mut entries: Vec<(&str, Vec<(&str, NbtTag)>)> = vec![(Self::CONTROL, vec![])];
        let rows: Vec<Vec<i32>> = Self::RENDER_CHECK
            .iter()
            .map(|row| {
                let mut ids = vec![0];
                for name in row.iter() {
                    entries.push((name, vec![]));
                    ids.push(entries.len() as i32 - 1);
                }
                ids
            })
            .collect();
        let step = Self::TINT_GAP + 1;
        let entity_row = rows.len() * step;
        let origin = [1000, 64, 5000];
        let mut extra: Vec<(usize, usize, i32)> = Vec::new();
        let mut positions: Vec<(usize, usize, NbtTag)> = Vec::new();
        let mut widest = rows.iter().map(Vec::len).max().unwrap_or(0) * step;
        for (x, z, name, states, data) in Self::block_entity_row(origin, entity_row) {
            entries.push((name, states));
            extra.push((x, z, entries.len() as i32 - 1));
            widest = widest.max(x + step);
            if let Some(data) = data {
                positions.push((x, z, data));
            }
        }
        let mut depth = entity_row + step;
        for (x, z, name, states, data) in Self::phase_three_rows(origin, entity_row + step) {
            entries.push((name, states));
            extra.push((x, z, entries.len() as i32 - 1));
            widest = widest.max(x + step);
            depth = depth.max(z + step);
            if let Some(data) = data {
                positions.push((x, z, data));
            }
        }
        let size = [widest, 1, depth];
        let cell = |x: usize, _y: usize, z: usize| -> i32 {
            if let Some((_, _, id)) = extra.iter().find(|(ex, ez, _)| *ex == x && *ez == z) {
                return *id;
            }
            if !x.is_multiple_of(step) || !z.is_multiple_of(step) || z / step >= rows.len() {
                return Self::NONE;
            }
            rows[z / step].get(x / step).copied().unwrap_or(Self::NONE)
        };
        let block_entities = NbtCompound::new(
            positions
                .into_iter()
                .map(|(x, z, data)| {
                    let key = Self::index(size, x, 0, z).to_string();
                    let value = NbtCompound::new(vec![("block_entity_data".to_string(), data)]);
                    (key, NbtTag::Compound(value))
                })
                .collect(),
        );
        Self::encode_with(size, origin, Self::palette_of(entries), block_entities, cell)
    }

    // The last row of the render check: block entities with real data, one per column from column 1 on. Each item is
    // (x, z, block name, states, block entity data). A bed foot sits one cell before its head on z and a double chest
    // takes two cells on x.
    #[allow(clippy::type_complexity)]
    fn block_entity_row(
        origin: [i32; 3],
        z: usize,
    ) -> Vec<(usize, usize, &'static str, Vec<(&'static str, NbtTag)>, Option<NbtTag>)> {
        let step = Self::TINT_GAP + 1;
        let wx = |x: usize| origin[0] + x as i32;
        let wz = |z: usize| origin[2] + z as i32;
        let string = |text: &str| NbtTag::String(text.to_string());
        let compound = |entries: Vec<(&str, NbtTag)>| {
            NbtTag::Compound(NbtCompound::new(entries.into_iter().map(|(name, tag)| (name.to_string(), tag)).collect()))
        };
        let data = |id: &str, x: usize, z: usize, extra: Vec<(&str, NbtTag)>| -> Option<NbtTag> {
            let mut entries = vec![
                ("id", string(id)),
                ("isMovable", NbtTag::Byte(1)),
                ("x", NbtTag::Int(wx(x))),
                ("y", NbtTag::Int(origin[1])),
                ("z", NbtTag::Int(wz(z))),
            ];
            entries.extend(extra);
            Some(compound(entries))
        };
        let item = |name: &str| {
            compound(vec![
                ("Count", NbtTag::Byte(1)),
                ("Damage", NbtTag::Short(0)),
                ("Name", string(name)),
                ("WasPickedUp", NbtTag::Byte(0)),
            ])
        };
        let side = |text: &str| {
            compound(vec![
                ("HideGlowOutline", NbtTag::Byte(0)),
                ("IgnoreLighting", NbtTag::Byte(0)),
                ("PersistFormatting", NbtTag::Byte(1)),
                ("SignTextColor", NbtTag::Int(-16_777_216)),
                ("Text", string(text)),
                ("TextOwner", string("")),
            ])
        };
        let north = || vec![("minecraft:cardinal_direction", string("north"))];
        let empty = || NbtTag::List(Vec::new());
        let mut row = Vec::new();
        let mut column = 1;
        let mut at = |count: usize| {
            let x = column * step;
            column += count;
            x
        };

        let x = at(1);
        let patterns = vec![
            compound(vec![("Color", NbtTag::Int(4)), ("Pattern", string("mr"))]),
            compound(vec![("Color", NbtTag::Int(11)), ("Pattern", string("bs"))]),
        ];
        row.push((
            x,
            z,
            "minecraft:standing_banner",
            vec![("ground_sign_direction", NbtTag::Int(0))],
            data(
                "Banner",
                x,
                z,
                vec![("Base", NbtTag::Int(1)), ("Type", NbtTag::Int(0)), ("Patterns", NbtTag::List(patterns))],
            ),
        ));

        let x = at(1);
        for (cell_z, head) in [(z - 1, 0), (z, 1)] {
            row.push((
                x,
                cell_z,
                "minecraft:bed",
                vec![
                    ("direction", NbtTag::Int(0)),
                    ("head_piece_bit", NbtTag::Byte(head)),
                    ("occupied_bit", NbtTag::Byte(0)),
                ],
                data("Bed", x, cell_z, vec![("color", NbtTag::Byte(14))]),
            ));
        }

        let x = at(1);
        row.push((
            x,
            z,
            "minecraft:standing_sign",
            vec![("ground_sign_direction", NbtTag::Int(0))],
            data(
                "Sign",
                x,
                z,
                vec![("FrontText", side("Stratum\nghost sign")), ("BackText", side("")), ("IsWaxed", NbtTag::Byte(0))],
            ),
        ));

        for facing in [0, 2] {
            let x = at(1);
            row.push((
                x,
                z,
                "minecraft:undyed_shulker_box",
                vec![],
                data("ShulkerBox", x, z, vec![("facing", NbtTag::Byte(facing)), ("Items", empty())]),
            ));
        }

        let x = at(1);
        let sherds = ["angler", "archer", "arms_up", "blade"]
            .iter()
            .map(|name| string(&format!("minecraft:{name}_pottery_sherd")))
            .collect();
        row.push((
            x,
            z,
            "minecraft:decorated_pot",
            vec![("direction", NbtTag::Int(0))],
            data("DecoratedPot", x, z, vec![("sherds", NbtTag::List(sherds))]),
        ));

        let x = at(2);
        for (cell_x, lead, other) in [(x, 1, x + 1), (x + 1, 0, x)] {
            row.push((
                cell_x,
                z,
                "minecraft:chest",
                north(),
                data(
                    "Chest",
                    cell_x,
                    z,
                    vec![
                        ("Findable", NbtTag::Byte(0)),
                        ("Items", empty()),
                        ("pairlead", NbtTag::Byte(lead)),
                        ("pairx", NbtTag::Int(wx(other))),
                        ("pairz", NbtTag::Int(wz(z))),
                    ],
                ),
            ));
        }

        let x = at(1);
        row.push((
            x,
            z,
            "minecraft:lectern",
            vec![("minecraft:cardinal_direction", string("north")), ("powered_bit", NbtTag::Byte(0))],
            data(
                "Lectern",
                x,
                z,
                vec![
                    ("book", item("minecraft:writable_book")),
                    ("hasBook", NbtTag::Byte(1)),
                    ("page", NbtTag::Int(0)),
                    ("totalPages", NbtTag::Int(1)),
                ],
            ),
        ));

        let x = at(1);
        row.push((
            x,
            z,
            "minecraft:campfire",
            vec![("minecraft:cardinal_direction", string("north")), ("extinguished", NbtTag::Byte(0))],
            data("Campfire", x, z, vec![("Item1", item("minecraft:beef")), ("ItemTime1", NbtTag::Int(0))]),
        ));

        let x = at(1);
        row.push((x, z, "minecraft:trapped_chest", north(), data("Chest", x, z, vec![("Items", empty())])));

        let x = at(1);
        row.push((x, z, "minecraft:ender_chest", north(), data("EnderChest", x, z, vec![])));

        let x = at(1);
        let conduit = vec![("Active", NbtTag::Byte(1)), ("Target", NbtTag::Long(-1))];
        row.push((x, z, "minecraft:conduit", vec![], data("Conduit", x, z, conduit)));

        let x = at(1);
        let bell_states =
            vec![("attachment", string("standing")), ("direction", NbtTag::Int(0)), ("toggle_bit", NbtTag::Byte(0))];
        let bell = vec![("Direction", NbtTag::Int(0)), ("Ringing", NbtTag::Byte(0)), ("Ticks", NbtTag::Int(0))];
        row.push((x, z, "minecraft:bell", bell_states, data("Bell", x, z, bell)));

        let x = at(1);
        let table = data("EnchantTable", x, z, vec![("rott", NbtTag::Float(0.0))]);
        row.push((x, z, "minecraft:enchanting_table", vec![], table));

        let x = at(1);
        row.push((x, z, "minecraft:copper_golem_statue", vec![], data("CopperGolemStatue", x, z, vec![])));
        row
    }

    // Rows after the block entity row, from `z` on: double chests along x (row 0), double chests along z (row 1, two
    // cells deep), standing skulls (row 2), then wall skulls, a chain and water (row 3). Chests: chest, trapped
    // chest and copper chest in the four facings; both halves name each other in world coordinates.
    #[allow(clippy::type_complexity)]
    fn phase_three_rows(
        origin: [i32; 3],
        z: usize,
    ) -> Vec<(usize, usize, &'static str, Vec<(&'static str, NbtTag)>, Option<NbtTag>)> {
        let step = Self::TINT_GAP + 1;
        let compound = |entries: Vec<(&str, NbtTag)>| {
            NbtTag::Compound(NbtCompound::new(entries.into_iter().map(|(name, tag)| (name.to_string(), tag)).collect()))
        };
        let data = |id: &str, x: usize, z: usize, extra: Vec<(&str, NbtTag)>| -> Option<NbtTag> {
            let mut entries = vec![
                ("id", NbtTag::String(id.to_string())),
                ("isMovable", NbtTag::Byte(1)),
                ("x", NbtTag::Int(origin[0] + x as i32)),
                ("y", NbtTag::Int(origin[1])),
                ("z", NbtTag::Int(origin[2] + z as i32)),
            ];
            entries.extend(extra);
            Some(compound(entries))
        };
        let mut row = Vec::new();
        let chests = ["minecraft:chest", "minecraft:trapped_chest", "minecraft:copper_chest"];
        let facings = ["north", "south", "east", "west"];
        for (axis_row, along_z) in [(0, false), (1, true)] {
            let cell_z = z + axis_row * step;
            let mut column = 1;
            for name in chests {
                for facing in facings {
                    let x = column * step;
                    column += 1;
                    for half in 0..2usize {
                        let (hx, hz) = if along_z { (x, cell_z + half) } else { (x + half, cell_z) };
                        let (ox, oz) = if along_z { (x, cell_z + 1 - half) } else { (x + 1 - half, cell_z) };
                        row.push((
                            hx,
                            hz,
                            name,
                            vec![("minecraft:cardinal_direction", NbtTag::String(facing.to_string()))],
                            data(
                                "Chest",
                                hx,
                                hz,
                                vec![
                                    ("Findable", NbtTag::Byte(0)),
                                    ("Items", NbtTag::List(Vec::new())),
                                    ("pairlead", NbtTag::Byte(i8::from(half == 0))),
                                    ("pairx", NbtTag::Int(origin[0] + ox as i32)),
                                    ("pairz", NbtTag::Int(origin[2] + oz as i32)),
                                ],
                            ),
                        ));
                    }
                }
            }
        }

        let skulls = [
            "minecraft:skeleton_skull",
            "minecraft:wither_skeleton_skull",
            "minecraft:zombie_head",
            "minecraft:player_head",
            "minecraft:creeper_head",
            "minecraft:dragon_head",
            "minecraft:piglin_head",
        ];
        let stand_z = z + 2 * step + 1;
        let wall_z = stand_z + step;
        let mut column = 1;
        for (kind, name) in skulls.into_iter().enumerate() {
            for turn in 0..4i8 {
                let x = column * step;
                column += 1;
                let rotation = if kind % 2 == 0 {
                    ("Rotation", NbtTag::Float(f32::from(turn) * 90.0))
                } else {
                    ("Rot", NbtTag::Byte(turn * 4))
                };
                row.push((
                    x,
                    stand_z,
                    name,
                    vec![("facing_direction", NbtTag::Int(1))],
                    data("Skull", x, stand_z, vec![("SkullType", NbtTag::Byte(kind as i8)), rotation]),
                ));
            }
        }
        for (kind, name) in skulls.into_iter().enumerate() {
            let x = (kind + 1) * step;
            row.push((
                x,
                wall_z,
                name,
                vec![("facing_direction", NbtTag::Int(2 + (kind % 4) as i32))],
                data("Skull", x, wall_z, vec![("SkullType", NbtTag::Byte(kind as i8))]),
            ));
        }
        row.push(((skulls.len() + 2) * step, wall_z, "minecraft:chain", vec![], None));
        for (offset, name) in [(4, "minecraft:water"), (5, "minecraft:water"), (6, "minecraft:lava")] {
            row.push(((skulls.len() + offset) * step, wall_z, name, vec![], None));
        }
        row
    }

    // Every sample with the file name it is written under: `large.mcstructure`, `standard.mcstructure`,
    // `tint.mcstructure`, `render-check.mcstructure`, `piece-<px>-<pz>.mcstructure`, then
    // `target-piece-<i>.mcstructure`.
    pub fn files() -> Result<Vec<(String, Vec<u8>)>, NbtWriteError> {
        let mut files = vec![
            ("large.mcstructure".to_string(), Self::large()?),
            ("standard.mcstructure".to_string(), Self::standard()?),
            ("tint.mcstructure".to_string(), Self::tint()?),
            ("render-check.mcstructure".to_string(), Self::render_check()?),
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
        Self::encode_with(size, origin, Self::palette(), NbtCompound::default(), cell)
    }

    fn encode_with(
        size: [usize; 3],
        origin: [i32; 3],
        palette: NbtTag,
        positions: NbtCompound,
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
            ("block_position_data".to_string(), NbtTag::Compound(positions)),
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
