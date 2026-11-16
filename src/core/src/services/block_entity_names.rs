// Names the blocks whose model the game draws with a block actor renderer.
pub struct BlockEntityNames;

impl BlockEntityNames {
    // True when the block named `name` needs a block actor ghost.
    pub fn has_block_actor(name: &str) -> bool {
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        matches!(
            name,
            "chest"
                | "trapped_chest"
                | "ender_chest"
                | "bed"
                | "standing_banner"
                | "wall_banner"
                | "conduit"
                | "decorated_pot"
                | "bell"
                | "enchanting_table"
                | "lectern"
                | "mob_spawner"
                | "trial_spawner"
                | "vault"
                | "campfire"
                | "soul_campfire"
                | "shelf"
                | "frame"
                | "glow_frame"
                | "suspicious_sand"
                | "suspicious_gravel"
                | "piston"
                | "sticky_piston"
        ) || Self::is_skull(name)
            || name.ends_with("copper_chest")
            || name.ends_with("shulker_box")
            || name.ends_with("_sign")
            || name.ends_with("_shelf")
            || name.ends_with("copper_golem_statue")
    }

    // True when the block named `name` is a chest that can pair into a double chest. An ender chest has no
    // large form.
    pub fn is_chest(name: &str) -> bool {
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        matches!(name, "chest" | "trapped_chest") || name.contains("copper_chest")
    }

    // True when the block named `name` is a skull or a head.
    pub fn is_skull(name: &str) -> bool {
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        matches!(
            name,
            "skeleton_skull"
                | "wither_skeleton_skull"
                | "zombie_head"
                | "player_head"
                | "creeper_head"
                | "dragon_head"
                | "piglin_head"
        )
    }
}
