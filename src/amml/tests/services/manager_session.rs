use alaydriem_stratum::services::ManagerSession;
use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amm_core::structs::lease::AcquireOutcome;
use amm_core::structs::registry::ModIndex;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::hud::HudMode;

use crate::support::{CoreSession, GhostScene, GhostSeen, Hud, Leases, Markers, PickerGuard};

#[test]
fn open_key_opens_with_the_overlay_and_input_leases() {
    let session = CoreSession::new();
    assert!(!session.shared().is_shown());

    session.open();

    assert!(session.shared().is_shown());
    assert_eq!(Leases::acquired(), [Capability::Overlay, Capability::Input]);
    assert!(Leases::released().is_empty());
}

#[test]
fn a_menu_entry_click_does_nothing() {
    let session = CoreSession::new();
    FakeTable::push_event(ModEvent::MenuEntryClicked { entry: 1 });

    session.tick(&CoreSession::in_world());

    assert!(!session.shared().is_shown());
    assert!(Leases::acquired().is_empty());
}

#[test]
fn open_key_toggles_the_library() {
    let session = CoreSession::new();
    FakeTable::push_event(CoreSession::OPEN_KEY);
    session.tick(&CoreSession::in_world());
    assert!(session.shared().is_shown());

    FakeTable::push_event(CoreSession::OPEN_KEY);
    session.tick(&CoreSession::in_world());

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);
}

#[test]
fn a_released_or_repeated_open_key_does_nothing() {
    let session = CoreSession::new();
    FakeTable::push_event(ModEvent::Keybind { action: 0, phase: KeybindPhase::Released });
    FakeTable::push_event(ModEvent::Keybind { action: 0, phase: KeybindPhase::Repeat });

    session.tick(&CoreSession::in_world());

    assert!(!session.shared().is_shown());
    assert!(Leases::acquired().is_empty());
}

#[test]
fn open_key_outside_a_world_does_nothing() {
    let session = CoreSession::new();
    FakeTable::push_event(CoreSession::OPEN_KEY);

    session.tick(&CoreSession::out_of_world());

    assert!(!session.shared().is_shown());
    assert!(Leases::acquired().is_empty());
}

#[test]
fn denied_overlay_does_not_open() {
    let session = CoreSession::new();
    FakeTable::set_acquire_outcome(Capability::Overlay, AcquireOutcome::Denied { holder: ModIndex(4) });
    FakeTable::with(|state| state.holder_id = Some("alaydriem-other"));

    session.open();

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::acquired(), [Capability::Overlay]);
    let denied = CoreSession::logged("overlay lease denied");
    assert_eq!(denied.len(), 1);
    assert!(denied[0].fields_json.contains("alaydriem-other"));
}

#[test]
fn a_denied_input_lease_still_opens_the_library() {
    let session = CoreSession::new();
    FakeTable::set_acquire_outcome(Capability::Input, AcquireOutcome::Denied { holder: ModIndex(5) });

    session.open();

    assert!(session.shared().is_shown());
    assert_eq!(Leases::acquired(), [Capability::Overlay, Capability::Input]);

    FakeTable::push_event(CoreSession::OPEN_KEY);
    session.tick(&CoreSession::in_world());

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::released(), [Capability::Overlay]);
}

#[test]
fn leaving_the_world_closes_the_library() {
    let session = CoreSession::new();
    session.open();

    session.tick(&CoreSession::out_of_world());

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);
}

#[test]
fn a_close_from_the_frame_releases_the_lease_on_the_next_tick() {
    let session = CoreSession::new();
    session.open();

    session.shared().request_close();
    assert!(!session.shared().is_shown());
    assert!(Leases::released().is_empty());
    session.tick(&CoreSession::in_world());

    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);
}

#[test]
fn a_revoked_overlay_closes_the_library() {
    let session = Leases::closed_by(ModEvent::Revoked { capability: Capability::Overlay, by: ModIndex(2) });

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);
    let closed = CoreSession::logged("library closed");
    assert_eq!(closed.len(), 1);
    assert!(closed[0].fields_json.contains("revoked"));
}

#[test]
fn a_suspended_overlay_closes_the_library_and_gives_the_lease_back() {
    let session = Leases::closed_by(ModEvent::Suspended { capability: Capability::Overlay, by: ModIndex(3) });

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);
    let closed = CoreSession::logged("library closed");
    assert_eq!(closed.len(), 1);
    assert!(closed[0].fields_json.contains("suspended"));
}

#[test]
fn a_disabled_mod_closes_the_library() {
    let session = Leases::closed_by(ModEvent::Disabled);

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);
    let closed = CoreSession::logged("library closed");
    assert_eq!(closed.len(), 1);
    assert!(closed[0].fields_json.contains("disabled"));
}

#[test]
fn events_for_other_capabilities_leave_the_library_open() {
    let session = Leases::closed_by(ModEvent::Suspended { capability: Capability::Camera, by: ModIndex(3) });

    assert!(session.shared().is_shown());
    assert!(Leases::released().is_empty());
}

#[test]
fn a_game_menu_closes_the_library() {
    let session = CoreSession::new();
    session.open();

    session.tick(&CoreSession::in_menu());

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);
    let closed = CoreSession::logged("library closed");
    assert_eq!(closed.len(), 1);
    assert!(closed[0].fields_json.contains("menu"));
}

// Escape outside a text field reaches the game, which opens the pause screen: the library then closes and
// holds nothing on later ticks.
#[test]
fn the_pause_screen_after_escape_leaves_nothing_held() {
    let session = CoreSession::new();
    session.open();

    session.tick(&CoreSession::in_menu());
    session.tick(&CoreSession::in_menu());
    session.tick(&CoreSession::in_world());

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::acquired(), [Capability::Overlay, Capability::Input]);
    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);
    assert_eq!(CoreSession::logged("library closed").len(), 1);
}

#[test]
fn open_key_while_a_game_menu_is_shown_does_nothing() {
    let session = CoreSession::new();
    FakeTable::push_event(CoreSession::OPEN_KEY);

    session.tick(&CoreSession::in_menu());

    assert!(!session.shared().is_shown());
    assert!(Leases::acquired().is_empty());
}

#[test]
fn open_and_close_are_logged_with_the_close_reason() {
    let session = CoreSession::new();
    session.open();
    assert_eq!(CoreSession::logged("library opened").len(), 1);

    FakeTable::push_event(CoreSession::OPEN_KEY);
    session.tick(&CoreSession::in_world());

    let closed = CoreSession::logged("library closed");
    assert_eq!(closed.len(), 1);
    assert!(closed[0].fields_json.contains("key"));
}

#[test]
fn menu_without_a_picker_still_closes() {
    let session = CoreSession::new();
    session.open();
    assert!(!PickerGuard::unwanted(&session));

    session.tick(&CoreSession::in_menu_at(2.0));

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);
}

#[test]
fn menu_during_the_picker_keeps_the_library_open() {
    let session = CoreSession::new();
    session.open();
    PickerGuard::start(&session, 1.5);

    session.tick(&CoreSession::in_menu_at(2.0));

    assert!(session.shared().is_shown());
    assert!(PickerGuard::unwanted(&session));
    assert!(Leases::released().is_empty());
}

#[test]
fn menu_right_after_the_picker_keeps_it_open_until_settled() {
    let session = CoreSession::new();
    session.open();
    PickerGuard::start(&session, 1.5);
    session.tick(&CoreSession::in_menu_at(2.0));
    PickerGuard::cancel(&session);

    session.tick(&CoreSession::in_menu_at(3.0));
    session.tick(&CoreSession::in_menu_at(3.5));

    assert!(session.shared().is_shown());
    assert!(PickerGuard::unwanted(&session));
    assert!(Leases::released().is_empty());
}

#[test]
fn guard_ends_on_the_first_tick_without_a_menu() {
    let session = CoreSession::new();
    session.open();
    PickerGuard::start(&session, 1.5);
    session.tick(&CoreSession::in_menu_at(2.0));
    PickerGuard::cancel(&session);
    session.tick(&CoreSession::in_menu_at(3.0));

    session.tick(&CoreSession::in_world_at(3.5));
    assert!(!PickerGuard::unwanted(&session));
    assert!(session.shared().is_shown());

    session.tick(&CoreSession::in_menu_at(4.0));
    assert!(!session.shared().is_shown());
    let closed = CoreSession::logged("library closed");
    assert_eq!(closed.len(), 1);
    assert!(closed[0].fields_json.contains("menu"));
}

#[test]
fn menu_after_the_settle_time_closes_then_reopens_once() {
    let session = CoreSession::new();
    session.open();
    PickerGuard::start(&session, 1.5);
    session.tick(&CoreSession::in_world_at(2.0));
    PickerGuard::cancel(&session);
    session.tick(&CoreSession::in_menu_at(3.0));
    session.tick(&CoreSession::in_menu_at(4.0));
    assert!(session.shared().is_shown());

    session.tick(&CoreSession::in_menu_at(3.0 + ManagerSession::PICKER_SETTLE_SECONDS + 0.1));

    assert!(!session.shared().is_shown());
    assert!(!PickerGuard::unwanted(&session));
    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);
    let closed = CoreSession::logged("library closed");
    assert!(closed[0].fields_json.contains("menu"));

    session.tick(&CoreSession::in_world_at(5.2));
    assert!(session.shared().is_shown());
    assert_eq!(Leases::acquired(), [Capability::Overlay, Capability::Input, Capability::Overlay, Capability::Input]);

    session.tick(&CoreSession::in_menu_at(5.3));
    session.tick(&CoreSession::in_world_at(5.4));
    assert!(!session.shared().is_shown());
    assert_eq!(Leases::acquired().len(), 4);
}

#[test]
fn the_open_key_cancels_a_pending_reopen() {
    let session = CoreSession::new();
    session.open();
    PickerGuard::start(&session, 1.5);
    session.tick(&CoreSession::in_world_at(2.0));
    PickerGuard::cancel(&session);
    session.tick(&CoreSession::in_menu_at(3.0));
    session.tick(&CoreSession::in_menu_at(6.0));
    assert!(!session.shared().is_shown());

    FakeTable::push_event(CoreSession::OPEN_KEY);
    session.tick(&CoreSession::in_menu_at(6.5));
    session.tick(&CoreSession::in_world_at(7.0));
    assert!(!session.shared().is_shown());
    assert_eq!(Leases::acquired().len(), 2);
}

#[test]
fn overlay_suspended_during_the_picker_hides_but_keeps_the_lease() {
    let session = CoreSession::new();
    session.open();
    PickerGuard::start(&session, 1.5);

    FakeTable::push_event(ModEvent::Suspended { capability: Capability::Overlay, by: ModIndex(3) });
    session.tick(&CoreSession::in_world_at(2.0));

    assert!(!session.shared().is_shown());
    assert!(Leases::released().is_empty());

    FakeTable::push_event(ModEvent::Resumed(Capability::Overlay));
    session.tick(&CoreSession::in_world_at(2.5));

    assert!(session.shared().is_shown());
    assert!(Leases::released().is_empty());
}

#[test]
fn a_taken_picker_request_is_unwanted_before_the_next_tick() {
    let session = CoreSession::new();
    session.open();
    session.shared().send(&mut vec![LibraryAction::RequestFilePicker(None)]);
    session.tick(&CoreSession::in_world_at(1.5));

    assert!(PickerGuard::take_request(&session));

    assert!(PickerGuard::unwanted(&session));
}

// Leaves the library closed by an expired guard with the reopen pending.
fn pending_reopen() -> CoreSession {
    let session = CoreSession::new();
    session.open();
    PickerGuard::start(&session, 1.5);
    session.tick(&CoreSession::in_world_at(2.0));
    PickerGuard::cancel(&session);
    session.tick(&CoreSession::in_menu_at(3.0));
    session.tick(&CoreSession::in_menu_at(6.0));
    assert!(!session.shared().is_shown());
    session
}

#[test]
fn a_reopen_after_the_pause_screen_keeps_the_ui_state_but_a_key_open_starts_fresh() {
    let session = pending_reopen();
    let opens = session.shared().opens();

    session.tick(&CoreSession::in_world_at(7.0));
    assert!(session.shared().is_shown());
    assert_eq!(session.shared().opens(), opens);

    FakeTable::push_event(CoreSession::OPEN_KEY);
    session.tick(&CoreSession::in_world_at(7.5));
    assert!(!session.shared().is_shown());
    FakeTable::push_event(CoreSession::OPEN_KEY);
    session.tick(&CoreSession::in_world_at(8.0));
    assert_eq!(session.shared().opens(), opens + 1);
}

#[test]
fn world_exit_clears_a_pending_reopen() {
    let session = pending_reopen();

    session.tick(&CoreSession::out_of_world());
    session.tick(&CoreSession::in_world_at(7.0));
    session.tick(&CoreSession::in_world_at(8.0));

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::acquired().len(), 2);
}

#[test]
fn disabled_clears_a_pending_reopen() {
    let session = pending_reopen();

    FakeTable::push_event(ModEvent::Disabled);
    session.tick(&CoreSession::in_menu_at(6.5));
    FakeTable::push_event(ModEvent::Enabled);
    session.tick(&CoreSession::in_world_at(7.0));
    session.tick(&CoreSession::in_world_at(8.0));

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::acquired().len(), 2);
}

#[test]
fn world_exit_during_the_picker_closes_without_reopen() {
    let session = CoreSession::new();
    session.open();
    PickerGuard::start(&session, 1.5);

    session.tick(&CoreSession::out_of_world());
    assert!(!session.shared().is_shown());
    assert_eq!(Leases::released(), [Capability::Input, Capability::Overlay]);

    PickerGuard::cancel(&session);
    session.tick(&CoreSession::in_world_at(3.0));
    session.tick(&CoreSession::in_world_at(4.0));

    assert!(!session.shared().is_shown());
    assert_eq!(Leases::acquired().len(), 2);
}

#[test]
fn world_draw_held_only_while_a_marker_is_active_and_shown() {
    let session = CoreSession::new();
    session.tick(&CoreSession::in_world());
    session.tick(&CoreSession::in_world());
    assert_eq!(Markers::world_draw(Leases::acquired()), 0);

    Markers::place(&session);
    assert_eq!(Markers::world_draw(Leases::acquired()), 1);
    session.tick(&CoreSession::in_world());
    assert_eq!(Markers::world_draw(Leases::acquired()), 1);
    assert_eq!(Markers::world_draw(Leases::released()), 0);

    session.shared().send(&mut vec![LibraryAction::DeactivateMarker]);
    session.tick(&CoreSession::in_world());

    assert_eq!(Markers::world_draw(Leases::released()), 1);
    assert_eq!(CoreSession::logged("world draw lease given back").len(), 1);
}

#[test]
fn leaving_the_world_gives_the_world_draw_lease_back() {
    let session = CoreSession::new();
    Markers::place(&session);

    session.tick(&CoreSession::out_of_world());

    assert_eq!(Markers::world_draw(Leases::released()), 1);
}

#[test]
fn suspended_world_draw_stops_the_ghost_and_keeps_the_marker() {
    let (session, seen) = GhostScene::start();
    assert_eq!(GhostScene::finish(&session, &seen, 480).unscaled_draws().len(), 1);

    FakeTable::push_event(ModEvent::Suspended { capability: Capability::WorldDraw, by: ModIndex(3) });
    session.tick(&CoreSession::in_world());

    // the ghost stops in this frame and keeps its meshes for the resume
    let stopped = GhostSeen::take(&session);
    assert!(stopped.draws.is_empty());
    assert!(!stopped.clear_all);
    assert!(stopped.frees.is_empty());
    assert_eq!(Markers::world_draw(Leases::released()), 1);
    assert!(session.shared().snapshot().markers.iter().any(|marker| marker.active));
    let lost = CoreSession::logged("world draw lease lost");
    assert_eq!(lost.len(), 1);
    assert!(lost[0].fields_json.contains("suspended"));

    FakeTable::push_event(ModEvent::Resumed(Capability::WorldDraw));
    session.tick(&CoreSession::in_world());

    let resumed = GhostSeen::take(&session);
    assert_eq!(resumed.unscaled_draws().len(), 1);
    assert_eq!(resumed.miniature_draws().len(), 1);
    assert!(resumed.jobs.is_empty() && !resumed.clear_all && resumed.palette.is_none());
}

#[test]
fn a_revoked_world_draw_clears_the_ghost() {
    let (session, seen) = GhostScene::start();
    GhostScene::report_built(&session, &seen, 480);
    session.tick(&CoreSession::in_world());
    assert_eq!(GhostSeen::take(&session).unscaled_draws().len(), 1);

    FakeTable::push_event(ModEvent::Revoked { capability: Capability::WorldDraw, by: ModIndex(2) });
    session.tick(&CoreSession::in_world());

    let cleared = GhostSeen::take(&session);
    assert!(cleared.draws.is_empty());
    assert!(cleared.clear_all);
}

#[test]
fn a_config_that_still_sets_the_old_opacity_changes_nothing() {
    let (session, _) = GhostScene::start();
    let before = GhostSeen::take(&session);
    assert!((before.opacity - 0.5).abs() < 1e-6);

    FakeTable::set_config("ghost {\n  opacity = 80\n}\n");
    FakeTable::push_event(ModEvent::SettingsChanged { paths: 1 });
    session.tick(&CoreSession::in_world());

    let after = GhostSeen::take(&session);
    assert!((after.opacity - 0.5).abs() < 1e-6);
    assert_eq!(after.generation, before.generation);
}

#[test]
fn world_draw_is_asked_for_again_after_suspension() {
    let (session, _) = GhostScene::start();
    FakeTable::push_event(ModEvent::Suspended { capability: Capability::WorldDraw, by: ModIndex(3) });
    session.tick(&CoreSession::in_world());
    session.tick(&CoreSession::in_world());
    assert_eq!(Markers::world_draw(Leases::acquired()), 1);

    let later = 1.0 + ManagerSession::WORLD_DRAW_RETRY_SECONDS;
    let seen = GhostScene::tick_until_jobs(&session, later);

    assert_eq!(Markers::world_draw(Leases::acquired()), 2);
    assert!(!seen.jobs.is_empty());
}

#[test]
fn a_revoked_world_draw_is_not_given_back_and_is_asked_for_again() {
    let session = CoreSession::new();
    Markers::place(&session);

    FakeTable::push_event(ModEvent::Revoked { capability: Capability::WorldDraw, by: ModIndex(2) });
    session.tick(&CoreSession::in_world());
    assert_eq!(Markers::world_draw(Leases::released()), 0);
    session.tick(&CoreSession::in_world_at(1.0 + ManagerSession::WORLD_DRAW_RETRY_SECONDS));

    assert_eq!(Markers::world_draw(Leases::acquired()), 2);
}

#[test]
fn a_resumed_world_draw_is_asked_for_at_once() {
    let session = CoreSession::new();
    Markers::place(&session);
    FakeTable::push_event(ModEvent::Suspended { capability: Capability::WorldDraw, by: ModIndex(3) });
    session.tick(&CoreSession::in_world());

    FakeTable::push_event(ModEvent::Resumed(Capability::WorldDraw));
    session.tick(&CoreSession::in_world());

    assert_eq!(Markers::world_draw(Leases::acquired()), 2);
}

#[test]
fn a_denied_world_draw_waits_and_logs_once() {
    let session = CoreSession::new();
    FakeTable::set_acquire_outcome(Capability::WorldDraw, AcquireOutcome::Denied { holder: ModIndex(4) });
    Markers::place(&session);
    session.tick(&CoreSession::in_world());
    assert_eq!(Markers::world_draw(Leases::acquired()), 1);

    session.tick(&CoreSession::in_world_at(1.0 + ManagerSession::WORLD_DRAW_RETRY_SECONDS));

    assert_eq!(Markers::world_draw(Leases::acquired()), 2);
    assert_eq!(CoreSession::logged("world draw lease denied").len(), 1);
}

#[test]
fn hud_lease_held_only_while_conditions_hold() {
    let session = CoreSession::new();
    session.tick(&CoreSession::in_world());
    session.tick(&CoreSession::in_world());
    assert_eq!(Hud::count(Leases::acquired(), Capability::Hud), 0);
    assert!(!Hud::shown(&session));

    Markers::place(&session);
    assert_eq!(Hud::count(Leases::acquired(), Capability::Hud), 1);
    assert!(Hud::shown(&session));
    session.tick(&CoreSession::in_world());
    assert_eq!(Hud::count(Leases::acquired(), Capability::Hud), 1);

    session.open();
    assert!(!Hud::shown(&session));
    assert_eq!(Hud::count(Leases::released(), Capability::Hud), 1);

    FakeTable::push_event(CoreSession::OPEN_KEY);
    session.tick(&CoreSession::in_world());
    assert!(Hud::shown(&session));
    assert_eq!(Hud::count(Leases::acquired(), Capability::Hud), 2);

    session.shared().send(&mut vec![LibraryAction::DeactivateMarker]);
    session.tick(&CoreSession::in_world());
    assert!(!Hud::shown(&session));
    assert_eq!(Hud::count(Leases::released(), Capability::Hud), 2);
}

#[test]
fn another_mods_input_lease_keeps_the_hud_lease_back() {
    let session = CoreSession::new();
    FakeTable::set_held_by_other(Capability::Input, true);
    Markers::place(&session);
    session.tick(&CoreSession::in_world());

    assert!(!Hud::shown(&session));

    FakeTable::set_held_by_other(Capability::Input, false);
    session.tick(&CoreSession::in_world());

    assert!(Hud::shown(&session));

    FakeTable::set_held_by_other(Capability::Input, true);
    session.tick(&CoreSession::in_world());

    assert!(!Hud::shown(&session));
    assert_eq!(Hud::count(Leases::released(), Capability::Hud), 1);
}

#[test]
fn another_mods_shared_overlay_does_not_stand_the_hud_down() {
    let session = CoreSession::new();
    FakeTable::set_held_by_other(Capability::Overlay, true);

    Markers::place(&session);

    assert!(Hud::shown(&session));
}

#[test]
fn menu_open_gives_the_hud_lease_back() {
    let session = CoreSession::new();
    Markers::place(&session);
    assert!(Hud::shown(&session));

    session.tick(&CoreSession::in_menu());

    assert!(!Hud::shown(&session));
    assert_eq!(Hud::count(Leases::released(), Capability::Hud), 1);

    session.tick(&CoreSession::in_world());

    assert!(Hud::shown(&session));
}

#[test]
fn leaving_the_world_gives_the_hud_lease_back() {
    let session = CoreSession::new();
    Markers::place(&session);

    session.tick(&CoreSession::out_of_world());

    assert!(!Hud::shown(&session));
    assert_eq!(Hud::count(Leases::released(), Capability::Hud), 1);
}

#[test]
fn a_disabled_mod_gives_the_hud_lease_back_and_keeps_it_back() {
    let session = CoreSession::new();
    Markers::place(&session);

    FakeTable::push_event(ModEvent::Disabled);
    session.tick(&CoreSession::in_world());
    session.tick(&CoreSession::in_world_at(10.0));

    assert!(!Hud::shown(&session));
    assert_eq!(Hud::count(Leases::released(), Capability::Hud), 1);
    assert_eq!(Hud::count(Leases::acquired(), Capability::Hud), 1);

    FakeTable::push_event(ModEvent::Enabled);
    session.tick(&CoreSession::in_world_at(11.0));

    assert!(Hud::shown(&session));
}

#[test]
fn a_revoked_hud_lease_is_asked_for_again_later() {
    let session = CoreSession::new();
    Markers::place(&session);

    FakeTable::push_event(ModEvent::Revoked { capability: Capability::Hud, by: ModIndex(2) });
    session.tick(&CoreSession::in_world());
    assert!(!Hud::shown(&session));
    assert_eq!(Hud::count(Leases::released(), Capability::Hud), 0);

    session.tick(&CoreSession::in_world_at(1.0 + ManagerSession::HUD_RETRY_SECONDS));
    assert!(Hud::shown(&session));
}

#[test]
fn a_denied_hud_lease_waits_and_logs_once() {
    let session = CoreSession::new();
    FakeTable::set_acquire_outcome(Capability::Hud, AcquireOutcome::Denied { holder: ModIndex(4) });
    Markers::place(&session);
    assert!(!Hud::shown(&session));
    assert_eq!(Hud::count(Leases::acquired(), Capability::Hud), 1);

    session.tick(&CoreSession::in_world_at(1.0 + ManagerSession::HUD_RETRY_SECONDS));

    assert_eq!(Hud::count(Leases::acquired(), Capability::Hud), 2);
    assert_eq!(CoreSession::logged("hud lease denied").len(), 1);
}

#[test]
fn hud_key_reaches_the_controller_while_held() {
    let session = CoreSession::new();
    Markers::place(&session);
    assert_eq!(Markers::world_draw(Leases::released()), 0);

    FakeTable::push_event(Hud::press(ManagerSession::HUD_LEFT_ACTION));
    session.tick(&CoreSession::in_world());

    // Left in Quick mode hides the ghost, so the marker no longer wants the world draw lease
    assert_eq!(Markers::world_draw(Leases::released()), 1);
}

#[test]
fn a_repeated_hud_key_acts_again_and_a_release_does_not() {
    let session = CoreSession::new();
    Markers::place(&session);

    FakeTable::push_event(ModEvent::Keybind { action: ManagerSession::HUD_LEFT_ACTION, phase: KeybindPhase::Released });
    session.tick(&CoreSession::in_world());
    assert_eq!(Markers::world_draw(Leases::released()), 0);

    FakeTable::push_event(ModEvent::Keybind { action: ManagerSession::HUD_LEFT_ACTION, phase: KeybindPhase::Repeat });
    session.tick(&CoreSession::in_world());
    assert_eq!(Markers::world_draw(Leases::released()), 1);
}

#[test]
fn a_released_hud_key_is_no_longer_held() {
    let session = CoreSession::new();
    Markers::place(&session);
    assert!(!Hud::held(&session));

    FakeTable::push_event(Hud::press(ManagerSession::HUD_RIGHT_ACTION));
    FakeTable::push_event(Hud::press(ManagerSession::HUD_UP_ACTION));
    session.tick(&CoreSession::in_world());
    assert!(Hud::held(&session));

    FakeTable::push_event(Hud::release(ManagerSession::HUD_RIGHT_ACTION));
    session.tick(&CoreSession::in_world());
    assert!(Hud::held(&session));

    FakeTable::push_event(Hud::release(ManagerSession::HUD_UP_ACTION));
    session.tick(&CoreSession::in_world());
    assert!(!Hud::held(&session));
}

#[test]
fn a_lost_hud_lease_lets_go_of_the_held_keys() {
    let session = CoreSession::new();
    Markers::place(&session);
    FakeTable::push_event(Hud::press(ManagerSession::HUD_RIGHT_ACTION));
    session.tick(&CoreSession::in_world());
    assert!(Hud::held(&session));

    FakeTable::push_event(ModEvent::Revoked { capability: Capability::Hud, by: ModIndex(2) });
    session.tick(&CoreSession::in_world());

    assert!(!Hud::held(&session));
}

#[test]
fn a_menu_opened_while_a_key_is_held_lets_go_of_it() {
    let session = CoreSession::new();
    Markers::place(&session);
    FakeTable::push_event(Hud::press(ManagerSession::HUD_UP_ACTION));
    session.tick(&CoreSession::in_world());
    assert!(Hud::held(&session));

    session.tick(&CoreSession::in_menu());

    assert!(!Hud::held(&session));
}

#[test]
fn the_mode_key_switches_the_hud_to_full_and_back() {
    let session = CoreSession::new();
    Markers::place(&session);
    assert_eq!(Hud::state(&session).mode, HudMode::Quick);

    FakeTable::push_event(Hud::press(ManagerSession::HUD_MODE_ACTION));
    session.tick(&CoreSession::in_world());
    assert_eq!(Hud::state(&session).mode, HudMode::Full);

    FakeTable::push_event(Hud::press(ManagerSession::HUD_MODE_ACTION));
    session.tick(&CoreSession::in_world());
    assert_eq!(Hud::state(&session).mode, HudMode::Quick);
}

#[test]
fn a_hud_key_is_ignored_while_the_lease_is_not_held() {
    let session = CoreSession::new();
    Markers::place(&session);
    session.tick(&CoreSession::in_menu());
    assert!(!Hud::shown(&session));

    FakeTable::push_event(Hud::press(ManagerSession::HUD_LEFT_ACTION));
    FakeTable::push_event(Hud::press(ManagerSession::HUD_MODE_ACTION));
    session.tick(&CoreSession::in_menu());

    assert_eq!(Markers::world_draw(Leases::released()), 0);
    assert_eq!(Hud::state(&session).mode, HudMode::Quick);
}

#[test]
fn library_holds_input_exclusively_while_open() {
    let session = CoreSession::new();
    session.open();
    assert_eq!(Hud::count(Leases::acquired(), Capability::Input), 1);
    assert_eq!(Hud::count(Leases::released(), Capability::Input), 0);
}

#[test]
fn input_is_released_on_a_key_close() {
    let session = CoreSession::new();
    session.open();

    FakeTable::push_event(CoreSession::OPEN_KEY);
    session.tick(&CoreSession::in_world());

    assert_eq!(Hud::count(Leases::released(), Capability::Input), 1);
}

#[test]
fn input_is_released_on_a_frame_close() {
    let session = CoreSession::new();
    session.open();

    session.shared().request_close();
    session.tick(&CoreSession::in_world());

    assert_eq!(Hud::count(Leases::released(), Capability::Input), 1);
}

#[test]
fn input_is_released_on_a_menu_close() {
    let session = CoreSession::new();
    session.open();

    session.tick(&CoreSession::in_menu());

    assert_eq!(Hud::count(Leases::released(), Capability::Input), 1);
}

#[test]
fn input_is_released_on_a_world_exit() {
    let session = CoreSession::new();
    session.open();

    session.tick(&CoreSession::out_of_world());

    assert_eq!(Hud::count(Leases::released(), Capability::Input), 1);
}

#[test]
fn input_is_released_by_the_panic_release() {
    let session = CoreSession::new();
    session.open();

    session.handle().with_state(|state| state.release_ui()).unwrap();

    assert_eq!(Hud::count(Leases::released(), Capability::Input), 1);
}

#[test]
fn input_is_released_when_the_mod_is_disabled() {
    let session = CoreSession::new();
    session.open();

    FakeTable::push_event(ModEvent::Disabled);
    session.tick(&CoreSession::in_world());

    assert_eq!(Hud::count(Leases::released(), Capability::Input), 1);
}

#[test]
fn a_closed_library_never_holds_input() {
    let session = CoreSession::new();
    Markers::place(&session);
    session.tick(&CoreSession::in_world());

    assert_eq!(Hud::count(Leases::acquired(), Capability::Input), 0);
}
