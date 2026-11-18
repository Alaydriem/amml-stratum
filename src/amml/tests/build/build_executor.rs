use alaydriem_stratum::StratumMod;
use alaydriem_stratum::build::BuildExecutor;
use alaydriem_stratum::detours::HookBody;
use alaydriem_stratum::types::{BuildCalls, GhostCalls, NbtLookup};
use amml_game::StopLatch;
use stratum_core::structs::build::{
    BuildConditions, BuildKindsRequest, BuildKindsResult, BuildProbeRequest, PaletteOutcome,
};

use crate::support::{FakeBuildCore, FakeBuildWorld, FakeGhost, FakeResolver};

fn calls() -> BuildCalls {
    let entries = StratumMod::resolve(&FakeResolver::new(&FakeResolver::values())).unwrap();
    let ghost = GhostCalls::new(&entries, FakeGhost::heap_new, FakeGhost::heap_delete).unwrap();
    BuildCalls::new(&entries, &ghost).unwrap()
}

fn commands(list: &[&str]) -> Vec<String> {
    list.iter().map(|text| (*text).to_string()).collect()
}

#[test]
fn the_conditions_go_to_the_core_each_step() {
    let mut executor = BuildExecutor::new(Some(calls()));
    let mut world = FakeBuildWorld::ready();
    let mut core = FakeBuildCore::default();

    executor.step(&mut world, &mut core);
    world.game_type = Some(0);
    executor.step(&mut world, &mut core);

    let all = BuildConditions { local: true, cheats: true, operator: true, creative: true };
    let not_creative = BuildConditions { creative: false, ..all };
    assert_eq!(core.conditions, vec![all, not_creative]);
}

#[test]
fn sends_each_due_command_once() {
    let mut executor = BuildExecutor::new(Some(calls()));
    let mut world = FakeBuildWorld::ready();
    let mut core = FakeBuildCore::default();
    core.commands.push_back(commands(&["/gamerule sendcommandfeedback false", "/fill 0 0 0 1 1 1 air"]));
    core.commands.push_back(commands(&["/setblock 0 0 0 stone"]));

    executor.step(&mut world, &mut core);
    executor.step(&mut world, &mut core);
    executor.step(&mut world, &mut core);

    assert_eq!(
        world.sent,
        commands(&["/gamerule sendcommandfeedback false", "/fill 0 0 0 1 1 1 air", "/setblock 0 0 0 stone"])
    );
    assert_eq!(core.send_failed, 0);
}

#[test]
fn a_send_error_reports_send_failed_and_sends_no_more() {
    let mut executor = BuildExecutor::new(Some(calls()));
    let mut world = FakeBuildWorld::ready();
    world.fail_send_at = Some(1);
    let mut core = FakeBuildCore::default();
    core.commands.push_back(commands(&["/a", "/b", "/c"]));

    executor.step(&mut world, &mut core);

    assert_eq!(world.sent, commands(&["/a"]));
    assert_eq!(core.send_failed, 1);
}

#[test]
fn with_the_feature_off_nothing_is_read_asked_or_sent() {
    let mut executor = BuildExecutor::new(None);
    let mut world = FakeBuildWorld::ready();
    let mut core = FakeBuildCore::default();
    core.commands.push_back(commands(&["/fill 0 0 0 1 1 1 air"]));
    core.probe = Some(BuildProbeRequest { entries: Vec::new(), quarters: 0 });

    executor.step(&mut world, &mut core);

    assert_eq!(core.conditions, vec![BuildConditions::default()]);
    assert!(world.sent.is_empty());
    assert_eq!((core.probe_takes, core.command_takes, core.send_failed), (0, 0, 0));
    assert_eq!(world.permission_calls.get(), 0);
}

#[test]
fn the_probe_is_answered_once_with_the_game_lookups() {
    let mut executor = BuildExecutor::new(Some(calls()));
    let mut world = FakeBuildWorld::ready();
    world.lookups.insert(vec![1], NbtLookup::Exact(FakeBuildWorld::block(1)));
    world.lookups.insert(vec![1, 2], NbtLookup::Exact(FakeBuildWorld::block(1)));
    let mut core = FakeBuildCore {
        probe: Some(BuildProbeRequest { entries: vec![(vec![1], vec![1, 2])], quarters: 0 }),
        ..FakeBuildCore::default()
    };

    executor.step(&mut world, &mut core);
    executor.step(&mut world, &mut core);

    assert_eq!(core.probe_answers.len(), 1);
    assert_eq!(core.probe_answers[0].outcomes, vec![PaletteOutcome::Exact]);
}

#[test]
fn a_kinds_request_is_scanned_and_answered_when_done() {
    let mut executor = BuildExecutor::new(Some(calls()));
    let mut world = FakeBuildWorld::ready();
    world.names.insert(1, "minecraft:stone".to_string());
    world.fill([0, 0, 0], [3, 3, 3], 1);
    let mut core =
        FakeBuildCore { kinds: Some(BuildKindsRequest { min: [0, 0, 0], max: [3, 3, 3] }), ..FakeBuildCore::default() };

    executor.step(&mut world, &mut core);

    assert_eq!(core.kinds_answers, vec![BuildKindsResult::Kinds(vec!["minecraft:stone".to_string()])]);
}

#[test]
fn a_kinds_request_over_unloaded_chunks_is_answered_not_loaded() {
    let mut executor = BuildExecutor::new(Some(calls()));
    let mut world = FakeBuildWorld::ready();
    world.names.insert(1, "minecraft:stone".to_string());
    world.fill([0, 0, 0], [3, 3, 3], 1);
    world.unloaded_chunks.insert((0, 0));
    let mut core =
        FakeBuildCore { kinds: Some(BuildKindsRequest { min: [0, 0, 0], max: [3, 3, 3] }), ..FakeBuildCore::default() };

    executor.step(&mut world, &mut core);

    assert_eq!(core.kinds_answers, vec![BuildKindsResult::NotLoaded]);
}

#[test]
fn a_panic_in_the_build_tick_does_not_cross_the_boundary() {
    let mut executor = BuildExecutor::new(Some(calls()));
    let mut world = FakeBuildWorld::ready();
    let mut core = FakeBuildCore { panic_on_conditions: true, ..FakeBuildCore::default() };
    let latch = StopLatch::new();

    let result = HookBody::before(|| 5u8, &latch, || executor.step(&mut world, &mut core));

    assert_eq!(result, 5);
    assert!(latch.raised());
}
