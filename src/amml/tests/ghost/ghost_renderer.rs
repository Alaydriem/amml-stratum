use amml_api::testing::FakeCanvas;
use amml_api::{BlockEntityCamera, Vec3};
use stratum_core::structs::ghost::{ChestPair, GhostBlockEntity, GhostReport, SkullPose};

use crate::support::RenderRig;

fn reports(rig: &RenderRig) -> Vec<GhostReport> {
    let mut seen = Vec::new();
    rig.ghost.drain(|report| seen.push(report.clone()));
    seen
}

fn canvas() -> FakeCanvas {
    FakeCanvas { camera_position: Some(Vec3 { x: 0.0, y: 70.0, z: 0.0 }), ..FakeCanvas::default() }
}

#[test]
fn a_job_is_built_with_the_mesher_and_drawn_one_cell_back_at_its_origin() {
    let rig = RenderRig::new();
    rig.ghost.publish(&RenderRig::frame(&["minecraft:stone"], Vec::new()));
    let canvas = canvas();

    rig.render(&canvas);

    let calls = canvas.calls.borrow().clone();
    assert!(calls.iter().any(|call| call == "mesher build cells=1 quads=6"), "{calls:?}");
    let draws: Vec<_> = calls.iter().filter(|call| call.starts_with("draw ")).collect();
    assert_eq!(draws.len(), 1, "{calls:?}");
    assert!(draws[0].ends_with(" 4 63 4 yaw=0"), "{calls:?}");
    let built: Vec<_> = reports(&rig).into_iter().flat_map(|report| report.built).collect();
    assert_eq!(built.len(), 1);
    assert!(built[0].ok);
    assert_eq!(built[0].bytes, 6 * 4 * 20);
}

#[test]
fn a_slot_outside_the_clip_matrix_is_culled_and_not_drawn() {
    let rig = RenderRig::new();
    rig.ghost.publish(&RenderRig::frame(&["minecraft:stone"], Vec::new()));
    // every point lands at clip x = 10 with w = 1: beyond the right plane
    let mut clip = [[0.0; 4]; 4];
    clip[3][0] = 10.0;
    clip[3][3] = 1.0;
    let canvas = FakeCanvas { clip_matrix: Some(clip), ..canvas() };

    rig.render(&canvas);

    assert!(!canvas.calls.borrow().iter().any(|call| call.starts_with("draw ")));
    assert_eq!(reports(&rig).iter().map(|report| report.culled).sum::<u32>(), 1);
}

#[test]
fn the_palette_is_resolved_once_per_palette_and_an_unknown_name_is_counted() {
    let rig = RenderRig::new();
    rig.ghost.publish(&RenderRig::frame(&["minecraft:stone", "minecraft:not_a_block"], Vec::new()));

    rig.render(&canvas());
    rig.ghost.publish(&RenderRig::repeat());
    rig.render(&canvas());

    let seen = reports(&rig);
    assert_eq!(seen.iter().filter(|report| report.palette_resolved).count(), 1);
    assert_eq!(seen.iter().map(|report| report.unknown_blocks).sum::<u32>(), 1);
}

#[test]
fn a_world_leave_forgets_the_meshes_the_loader_freed() {
    let rig = RenderRig::new();
    rig.ghost.publish(&RenderRig::frame(&["minecraft:stone"], Vec::new()));
    rig.render(&canvas());

    let held = rig.renderer.lock().unwrap().world_left(&rig.game.ctx().block_entities());

    assert_eq!(held, 1);
    assert_eq!(rig.renderer.lock().unwrap().meshes(), 0);
}

#[test]
fn a_stale_mesh_drops_the_slots_and_asks_core_to_build_again() {
    let rig = RenderRig::new();
    rig.ghost.publish(&RenderRig::frame(&["minecraft:stone"], Vec::new()));
    rig.render(&canvas());
    reports(&rig);

    rig.game.leave_mesh_world();
    rig.ghost.publish(&RenderRig::repeat());
    rig.render(&canvas());

    assert_eq!(rig.renderer.lock().unwrap().meshes(), 0);
    assert!(reports(&rig).iter().any(|report| report.world_reset));
}

#[test]
fn a_large_chest_lead_is_made_as_a_pair_and_drawn_in_the_player_pass_only() {
    let rig = RenderRig::new();
    let lead = GhostBlockEntity::new(13, GhostBlockEntity::NONE, Some(ChestPair::PlusX), None);
    rig.ghost.publish(&RenderRig::frame(&["minecraft:chest"], vec![lead]));
    rig.render(&canvas());

    let other = rig.pass(BlockEntityCamera::Other);
    let player = rig.pass(BlockEntityCamera::Player);

    assert_eq!((other, player), (0, 1));
    let calls = rig.game.block_entities().calls();
    assert!(calls.iter().any(|call| call == "create kind=chest nbt=0 pair=1,0,0"), "{calls:?}");
    assert!(calls.iter().any(|call| call.starts_with("draw ") && call.ends_with(" 5,64,5")), "{calls:?}");
}

#[test]
fn a_floor_skull_is_made_with_its_turn_in_sixteenths() {
    let rig = RenderRig::new();
    let skull = GhostBlockEntity::new(13, GhostBlockEntity::NONE, None, Some(SkullPose::new(1, 90.0)));
    rig.ghost.publish(&RenderRig::frame(&["minecraft:skeleton_skull"], vec![skull]));
    rig.render(&canvas());

    rig.pass(BlockEntityCamera::Player);

    let calls = rig.game.block_entities().calls();
    assert!(calls.iter().any(|call| call.starts_with("create kind=skull") && call.ends_with("skull=1,4")), "{calls:?}");
}

#[test]
fn the_ghost_of_one_key_is_made_once_across_passes() {
    let rig = RenderRig::new();
    let lead = GhostBlockEntity::new(13, GhostBlockEntity::NONE, Some(ChestPair::PlusX), None);
    rig.ghost.publish(&RenderRig::frame(&["minecraft:chest"], vec![lead]));
    rig.render(&canvas());

    for _ in 0..3 {
        rig.ghost.publish(&RenderRig::repeat());
        rig.render(&canvas());
        rig.pass(BlockEntityCamera::Player);
    }

    let creates = rig.game.block_entities().calls().into_iter().filter(|call| call.starts_with("create")).count();
    assert_eq!(creates, 1);
    assert_eq!(rig.renderer.lock().unwrap().block_entities(), 1);
}

#[test]
fn a_world_whose_air_is_found_late_is_tried_again_and_built_once_found() {
    use amml_api::testing::{FakeClient, FakeWorld};
    let rig = RenderRig::new();
    let found = crate::support::StratumRig::world();
    rig.game.set_client(FakeClient {
        local_player: true,
        world_active: true,
        world: Some(FakeWorld::default()),
        ..FakeClient::default()
    });
    rig.ghost.publish(&RenderRig::frame(&["minecraft:stone"], Vec::new()));
    rig.render(&canvas());
    assert_eq!(rig.renderer.lock().unwrap().meshes(), 0);
    reports(&rig);

    rig.game.set_client(FakeClient {
        local_player: true,
        world_active: true,
        world: Some(found),
        ..FakeClient::default()
    });
    for _ in 0..60 {
        rig.ghost.publish(&RenderRig::repeat());
        rig.render(&canvas());
    }
    assert!(reports(&rig).iter().any(|report| report.world_reset), "core is asked for a new generation");
    let mut again = RenderRig::frame(&["minecraft:stone"], Vec::new());
    again.generation = 2;
    rig.ghost.publish(&again);
    rig.render(&canvas());

    assert_eq!(rig.renderer.lock().unwrap().meshes(), 1);
}

#[test]
fn a_dimension_change_frees_the_meshes_draws_no_old_ghost_and_asks_core_for_a_new_generation() {
    let rig = RenderRig::new();
    let lead = GhostBlockEntity::new(13, GhostBlockEntity::NONE, Some(ChestPair::PlusX), None);
    rig.ghost.publish(&RenderRig::frame(&["minecraft:chest"], vec![lead]));
    rig.render(&canvas());
    reports(&rig);

    rig.renderer.lock().unwrap().dimension_changing();
    let drawn_before_the_frame = rig.pass(BlockEntityCamera::Player);
    rig.ghost.publish(&RenderRig::repeat());
    let canvas = canvas();
    rig.render(&canvas);

    assert_eq!(drawn_before_the_frame, 0, "the pass of the changing frame draws no old ghost");
    assert_eq!(canvas.frees.borrow().len(), 1);
    assert!(reports(&rig).iter().any(|report| report.world_reset));
}

#[test]
fn a_stale_block_entity_ghost_is_made_again_after_core_builds_again() {
    let rig = RenderRig::new();
    let lead = GhostBlockEntity::new(13, GhostBlockEntity::NONE, Some(ChestPair::PlusX), None);
    rig.ghost.publish(&RenderRig::frame(&["minecraft:chest"], vec![lead]));
    rig.render(&canvas());
    rig.pass(BlockEntityCamera::Player);
    reports(&rig);

    rig.game.leave_block_entity_world();
    rig.pass(BlockEntityCamera::Player);

    assert_eq!(rig.renderer.lock().unwrap().block_entities(), 0, "the freed ghosts are dropped");
    rig.ghost.publish(&RenderRig::repeat());
    rig.render(&canvas());
    assert!(reports(&rig).iter().any(|report| report.world_reset));
}
