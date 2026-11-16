use std::sync::Arc;

use stratum_core::services::{ChestPairer, GhostAssembler, NbtWriter};
use stratum_core::structs::ghost::{ChestPair, ChestRole, GhostBuild, PlacedStructure};
use stratum_core::structs::marker::Rotation;
use stratum_core::structs::nbt::NbtTag;
use stratum_core::structs::structure::{
    BlockEntities, BlockLayer, BlockState, BlockStates, SparseLayer, StateValue, Structure, StructureSize,
};

// Rows of chests along X cut at world (100, 64, 100), their pair keys in world coordinates.
struct Chests;

impl Chests {
    const ORIGIN: [i32; 3] = [100, 64, 100];

    fn block(name: &str) -> BlockState {
        Self::facing(name, "north")
    }

    // A block of `name` whose `minecraft:cardinal_direction` is `facing`.
    fn facing(name: &str, facing: &str) -> BlockState {
        let facing = StateValue::Text(facing.to_string());
        BlockState::new(
            format!("minecraft:{name}"),
            BlockStates::new(vec![("minecraft:cardinal_direction".to_string(), facing)]),
        )
    }

    // The NBT of a chest of the row that pairs with cell `other`. `lead` is the `pairlead` byte.
    fn nbt(other: i32, lead: Option<i8>) -> Vec<u8> {
        let mut tags = vec![
            ("id", NbtTag::String("Chest".to_string())),
            ("pairx", NbtTag::Int(Self::ORIGIN[0] + other)),
            ("pairz", NbtTag::Int(Self::ORIGIN[2])),
        ];
        if let Some(lead) = lead {
            tags.push(("pairlead", NbtTag::Byte(lead)));
        }
        NbtWriter::compound("", &tags).unwrap()
    }

    // A build of one row. `blocks` is the palette; cell `n` holds palette entry `n`. `nbt` is (cell, bytes).
    fn build(blocks: Vec<BlockState>, nbt: Vec<(u32, Vec<u8>)>, rotation: Rotation) -> GhostBuild {
        let count = blocks.len() as u32;
        let mut block_entities = BlockEntities::new();
        for (cell, bytes) in nbt {
            block_entities.insert(cell, bytes);
        }
        let structure = Arc::new(Structure {
            size: StructureSize::new(count, 1, 1),
            world_origin: Some(Self::ORIGIN),
            palette: blocks,
            primary: BlockLayer::new((0..count as u16).collect()),
            secondary: SparseLayer::default(),
            block_entities,
        });
        let turned = match rotation {
            Rotation::R0 => Arc::clone(&structure),
            rotation => Arc::new(GhostAssembler::turn(&structure, rotation)),
        };
        let placed = [PlacedStructure { structure, origin: Self::ORIGIN }];
        GhostAssembler::build(&placed, &[turned], Self::ORIGIN, rotation, 1, None).unwrap()
    }

    // The role of every cell of the build in X then Z order.
    fn roles(build: &GhostBuild) -> Vec<ChestRole> {
        let mut roles = Vec::new();
        for x in build.world_min[0]..=build.world_max[0] {
            for z in build.world_min[2]..=build.world_max[2] {
                roles.push(ChestPairer::pair(build, [x, Self::ORIGIN[1], z]));
            }
        }
        roles
    }
}

#[test]
fn a_chest_pair_has_one_lead_and_one_follower() {
    let chest = Chests::block("chest");
    let blocks = || vec![chest.clone(), chest.clone()];

    let first = Chests::build(blocks(), vec![(0, Chests::nbt(1, Some(1))), (1, Chests::nbt(0, Some(0)))], Rotation::R0);
    assert_eq!(Chests::roles(&first), [ChestRole::Lead(ChestPair::PlusX), ChestRole::Follower]);

    let second =
        Chests::build(blocks(), vec![(0, Chests::nbt(1, Some(0))), (1, Chests::nbt(0, Some(1)))], Rotation::R0);
    assert_eq!(Chests::roles(&second), [ChestRole::Follower, ChestRole::Lead(ChestPair::MinusX)]);

    let both = Chests::build(blocks(), vec![(0, Chests::nbt(1, Some(1))), (1, Chests::nbt(0, Some(1)))], Rotation::R0);
    assert_eq!(Chests::roles(&both), [ChestRole::Lead(ChestPair::PlusX), ChestRole::Follower]);

    let neither = Chests::build(blocks(), vec![(0, Chests::nbt(1, None)), (1, Chests::nbt(0, None))], Rotation::R0);
    assert_eq!(Chests::roles(&neither), [ChestRole::Lead(ChestPair::PlusX), ChestRole::Follower]);
}

#[test]
fn the_pair_direction_turns_with_the_ghost() {
    let chest = Chests::block("chest");
    let expected = [ChestPair::PlusX, ChestPair::PlusZ, ChestPair::MinusX, ChestPair::MinusZ];
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        let build = Chests::build(
            vec![chest.clone(), chest.clone()],
            vec![(0, Chests::nbt(1, Some(1))), (1, Chests::nbt(0, Some(0)))],
            rotation,
        );

        let roles = Chests::roles(&build);

        let lead = ChestRole::Lead(expected[usize::from(rotation.quarters())]);
        assert_eq!(roles.iter().filter(|role| **role == lead).count(), 1, "{rotation:?}: {roles:?}");
        assert_eq!(roles.iter().filter(|role| **role == ChestRole::Follower).count(), 1, "{rotation:?}: {roles:?}");
        assert_eq!(roles.len(), 2);
    }
}

#[test]
fn a_pair_whose_partner_is_missing_draws_both_single() {
    let chest = Chests::block("chest");
    let stone = Chests::block("stone");
    let west = BlockState::new(
        "minecraft:chest".to_string(),
        BlockStates::new(vec![("facing_direction".to_string(), StateValue::Int(4))]),
    );
    let cases = [
        // the partner cell is not a chest
        (vec![chest.clone(), stone], vec![(0, Chests::nbt(1, Some(1)))]),
        // the partner is a chest that does not name this one
        (vec![chest.clone(), chest.clone()], vec![(0, Chests::nbt(1, Some(1))), (1, Chests::nbt(5, Some(0)))]),
        // the partner is a chest with no NBT
        (vec![chest.clone(), chest.clone()], vec![(0, Chests::nbt(1, Some(1)))]),
        // the partner is another kind of chest
        (
            vec![chest.clone(), Chests::block("trapped_chest")],
            vec![(0, Chests::nbt(1, Some(1))), (1, Chests::nbt(0, Some(0)))],
        ),
        // the NBT names a cell that is not beside the chest
        (
            vec![chest.clone(), chest.clone(), chest.clone()],
            vec![(0, Chests::nbt(2, Some(1))), (2, Chests::nbt(0, Some(0)))],
        ),
        // the partner is in front of the chest: both face east, the pair runs along X
        (
            vec![Chests::facing("chest", "east"), Chests::facing("chest", "east")],
            vec![(0, Chests::nbt(1, Some(1))), (1, Chests::nbt(0, Some(0)))],
        ),
        // the partner is behind the chest, its facing given as `facing_direction` 4 (west)
        (vec![west.clone(), west], vec![(0, Chests::nbt(1, Some(1))), (1, Chests::nbt(0, Some(0)))]),
    ];
    for (index, (blocks, nbt)) in cases.into_iter().enumerate() {
        let count = blocks.len();
        let build = Chests::build(blocks, nbt, Rotation::R0);

        assert_eq!(Chests::roles(&build), vec![ChestRole::Single; count], "case {index}");
    }
}

#[test]
fn a_pair_beside_the_turned_facing_pairs_at_every_quarter() {
    for (facing, rotation) in [("south", Rotation::R0), ("east", Rotation::R90), ("north", Rotation::R180)] {
        let chest = Chests::facing("chest", facing);
        let build = Chests::build(
            vec![chest.clone(), chest],
            vec![(0, Chests::nbt(1, Some(1))), (1, Chests::nbt(0, Some(0)))],
            rotation,
        );

        let roles = Chests::roles(&build);

        let leads = roles.iter().filter(|role| matches!(role, ChestRole::Lead(_))).count();
        let expected = usize::from(facing != "east");
        assert_eq!(leads, expected, "{facing} at {rotation:?}: {roles:?}");
    }
}

#[test]
fn trapped_and_copper_chests_pair_like_chests() {
    for name in ["trapped_chest", "copper_chest", "waxed_oxidized_copper_chest", "exposed_copper_chest"] {
        let block = Chests::block(name);
        let build = Chests::build(
            vec![block.clone(), block],
            vec![(0, Chests::nbt(1, Some(1))), (1, Chests::nbt(0, Some(0)))],
            Rotation::R0,
        );

        assert_eq!(Chests::roles(&build), [ChestRole::Lead(ChestPair::PlusX), ChestRole::Follower], "{name}");
    }

    let ender = Chests::block("ender_chest");
    let build = Chests::build(
        vec![ender.clone(), ender],
        vec![(0, Chests::nbt(1, Some(1))), (1, Chests::nbt(0, Some(0)))],
        Rotation::R0,
    );
    assert_eq!(Chests::roles(&build), [ChestRole::Single, ChestRole::Single]);
}
