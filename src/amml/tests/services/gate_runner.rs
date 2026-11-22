use alaydriem_stratum::services::GateRunner;
use tempfile::TempDir;

fn instance(version: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let manifest = format!("<Package><Identity Name=\"Microsoft.MinecraftUWP\" Version=\"{version}\" /></Package>");
    std::fs::write(dir.path().join("AppxManifest.xml"), manifest).unwrap();
    dir
}

#[test]
fn the_skull_shape_follows_the_game_version_of_the_instance() {
    assert_eq!(GateRunner::shape_of(instance("1.26.6301.0").path()), "26.63");
    assert_eq!(GateRunner::shape_of(instance("1.26.7026.0").path()), "70.x");
    assert_eq!(GateRunner::shape_of(TempDir::new().unwrap().path()), "unknown");
}

#[test]
fn a_lookup_that_fails_before_the_world_is_ready_is_tried_again_each_second() {
    use amml_api::testing::{FakeClient, FakeGame, FakeWorld};
    let game = FakeGame::new();
    for area in crate::support::StratumRig::areas() {
        game.offer_area(area);
    }
    game.open_gate();
    game.set_epoch(1);
    let client =
        |world| FakeClient { local_player: true, world_active: true, world: Some(world), ..FakeClient::default() };
    game.set_client(client(FakeWorld::default()));
    let ctx = game.ctx();
    let mut gate = GateRunner::new(ctx.log(), None);
    let (blocks, entities) = (ctx.blocks(), ctx.block_entities());

    gate.tick(0.0, [0, 64, 0], None, &blocks, &entities, None);
    game.set_client(client(crate::support::StratumRig::world()));
    gate.tick(0.5, [0, 64, 0], None, &blocks, &entities, None);
    let early = game.logs().iter().filter(|line| line.contains("from_state")).count();
    gate.tick(1.0, [0, 64, 0], None, &blocks, &entities, None);

    assert_eq!(early, 0, "no line before the step succeeds or times out");
    let ok: Vec<_> = game.logs().into_iter().filter(|line| line.contains("gate block from_state ok")).collect();
    assert_eq!(ok.len(), 1, "{ok:?}");
    assert!(ok[0].contains("tries=2"), "{ok:?}");
}
