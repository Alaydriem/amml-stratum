use stratum_core::services::BlockEntityNames;

#[test]
fn each_block_actor_family_is_named() {
    let named = [
        "minecraft:chest",
        "minecraft:trapped_chest",
        "minecraft:ender_chest",
        "minecraft:copper_chest",
        "minecraft:exposed_copper_chest",
        "minecraft:weathered_copper_chest",
        "minecraft:oxidized_copper_chest",
        "minecraft:waxed_oxidized_copper_chest",
        "minecraft:undyed_shulker_box",
        "minecraft:red_shulker_box",
        "minecraft:bed",
        "minecraft:standing_banner",
        "minecraft:wall_banner",
        "minecraft:standing_sign",
        "minecraft:wall_sign",
        "minecraft:spruce_standing_sign",
        "minecraft:birch_wall_sign",
        "minecraft:oak_hanging_sign",
        "minecraft:crimson_wall_hanging_sign",
        "minecraft:conduit",
        "minecraft:decorated_pot",
        "minecraft:bell",
        "minecraft:enchanting_table",
        "minecraft:lectern",
        "minecraft:mob_spawner",
        "minecraft:trial_spawner",
        "minecraft:vault",
        "minecraft:campfire",
        "minecraft:soul_campfire",
        "minecraft:shelf",
        "minecraft:oak_shelf",
        "minecraft:frame",
        "minecraft:glow_frame",
        "minecraft:suspicious_sand",
        "minecraft:suspicious_gravel",
        "minecraft:copper_golem_statue",
        "minecraft:waxed_exposed_copper_golem_statue",
        "minecraft:piston",
        "minecraft:sticky_piston",
    ];
    for name in named {
        assert!(BlockEntityNames::has_block_actor(name), "{name} should be a block actor name");
    }
}

#[test]
fn skulls_are_block_actor_names() {
    for name in [
        "minecraft:skeleton_skull",
        "minecraft:wither_skeleton_skull",
        "minecraft:zombie_head",
        "minecraft:player_head",
        "minecraft:creeper_head",
        "minecraft:dragon_head",
        "minecraft:piglin_head",
    ] {
        assert!(BlockEntityNames::has_block_actor(name), "{name} should be a block actor name");
        assert!(BlockEntityNames::is_skull(name), "{name} should be a skull");
    }
    for name in ["minecraft:skull", "minecraft:stone", "minecraft:copper_block", "minecraft:bookshelf"] {
        assert!(!BlockEntityNames::has_block_actor(name), "{name} should not be a block actor name");
        assert!(!BlockEntityNames::is_skull(name), "{name} should not be a skull");
    }
}

#[test]
fn the_chest_family_is_chests_but_not_ender_chests() {
    for name in ["minecraft:chest", "trapped_chest", "minecraft:copper_chest", "minecraft:waxed_weathered_copper_chest"]
    {
        assert!(BlockEntityNames::is_chest(name), "{name} should be a chest");
    }
    for name in ["minecraft:ender_chest", "minecraft:barrel", "minecraft:copper_block"] {
        assert!(!BlockEntityNames::is_chest(name), "{name} should not be a chest");
    }
}
