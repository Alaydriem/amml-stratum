use crate::structs::block::Face;
use crate::structs::preview::{BlockShape, Occlusion};
use crate::structs::structure::{BlockState, StateValue};

// Picks the preview shape of a block from its name and states. The game hard-codes these shapes; the preview
// knows a few by name and draws every other block as a full cube.
pub struct ShapeRules;

impl ShapeRules {
    const HIDDEN: [&'static str; 5] = ["air", "structure_void", "barrier", "light_block", "light"];
    const CROSS: [&'static str; 48] = [
        "short_grass",
        "tall_grass",
        "tallgrass",
        "fern",
        "large_fern",
        "short_dry_grass",
        "tall_dry_grass",
        "deadbush",
        "dead_bush",
        "bush",
        "firefly_bush",
        "dandelion",
        "poppy",
        "blue_orchid",
        "allium",
        "azure_bluet",
        "red_tulip",
        "orange_tulip",
        "white_tulip",
        "pink_tulip",
        "oxeye_daisy",
        "cornflower",
        "lily_of_the_valley",
        "wither_rose",
        "torchflower",
        "open_eyeblossom",
        "closed_eyeblossom",
        "sunflower",
        "lilac",
        "rose_bush",
        "peony",
        "double_plant",
        "red_flower",
        "yellow_flower",
        "brown_mushroom",
        "red_mushroom",
        "crimson_fungus",
        "warped_fungus",
        "crimson_roots",
        "warped_roots",
        "nether_sprouts",
        "sweet_berry_bush",
        "wheat",
        "carrots",
        "potatoes",
        "beetroot",
        "seagrass",
        "web",
    ];

    pub fn shape(block: &BlockState) -> (BlockShape, Occlusion) {
        let name = block.name.strip_prefix("minecraft:").unwrap_or(&block.name);
        if Self::HIDDEN.contains(&name) || name.starts_with("light_block") {
            return (BlockShape::Hidden, Occlusion::None);
        }
        if Self::CROSS.contains(&name)
            || name.ends_with("_sapling")
            || name.ends_with("torch")
            || name == "reeds"
            || name == "sugar_cane"
            || name == "kelp"
            || name == "cobweb"
        {
            return (BlockShape::Cross, Occlusion::None);
        }
        if name.ends_with("_slab") && !name.contains("double_slab") {
            let top = matches!(block.states.get("minecraft:vertical_half"), Some(StateValue::Text(half)) if half == "top")
                || Self::flag(block, "top_slot_bit");
            let (min, max) = if top { ([0, 8, 0], [16, 16, 16]) } else { ([0, 0, 0], [16, 8, 16]) };
            return (BlockShape::Box { min, max }, Occlusion::None);
        }
        if name.ends_with("_stairs") {
            let high_side = match block.states.get("weirdo_direction") {
                Some(StateValue::Int(1)) => Face::West,
                Some(StateValue::Int(2)) => Face::South,
                Some(StateValue::Int(3)) => Face::North,
                _ => Face::East,
            };
            return (
                BlockShape::Stairs { upside_down: Self::flag(block, "upside_down_bit"), high_side },
                Occlusion::None,
            );
        }
        if name.ends_with("_carpet") || name == "carpet" {
            return (BlockShape::Box { min: [0, 0, 0], max: [16, 1, 16] }, Occlusion::None);
        }
        if name.ends_with("rail") || name == "redstone_wire" || name.ends_with("_pressure_plate") {
            return (BlockShape::Box { min: [0, 0, 0], max: [16, 1, 16] }, Occlusion::None);
        }
        if name.ends_with("_fence") || name == "fence" {
            return (BlockShape::Box { min: [6, 0, 6], max: [10, 16, 10] }, Occlusion::None);
        }
        if name.ends_with("_wall") && !name.contains("sign") && !name.contains("banner") {
            return (BlockShape::Box { min: [4, 0, 4], max: [12, 16, 12] }, Occlusion::None);
        }
        if name.ends_with("_pane") || name == "glass_pane" || name == "iron_bars" {
            return (BlockShape::Box { min: [7, 0, 7], max: [9, 16, 9] }, Occlusion::None);
        }
        if name.ends_with("leaves") || name.contains("leaves_") || name == "leaves" || name == "leaves2" {
            return (BlockShape::Cube, Occlusion::None);
        }
        if name.contains("glass")
            || name.contains("ice")
            || name.contains("water")
            || name.contains("lava")
            || name == "slime"
            || name == "honey_block"
        {
            return (BlockShape::Cube, Occlusion::Group(Self::group(name)));
        }
        (BlockShape::Cube, Occlusion::Opaque)
    }

    fn flag(block: &BlockState, state: &str) -> bool {
        match block.states.get(state) {
            Some(StateValue::Bool(value)) => *value,
            Some(StateValue::Int(value)) => *value != 0,
            _ => false,
        }
    }

    // A group id from the name, so blocks with the same name hide each other's shared faces. A collision only
    // hides a face between two different see-through blocks.
    fn group(name: &str) -> u16 {
        let base = name.strip_prefix("flowing_").unwrap_or(name);
        let hash = base.bytes().fold(0x811c_9dc5_u32, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193));
        (hash & 0xffff) as u16
    }
}
