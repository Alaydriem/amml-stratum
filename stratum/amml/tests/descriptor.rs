use alaydriem_stratum::services::ManagerSession;
use amm_core::services::DescriptorChecker;
use amm_core::services::settings::SchemaChecker;
use amm_core::structs::descriptor::{ApplyMode, Capability, Chord, ClaimMode, KeyScope};
use amm_core::structs::settings::HclDocument;
use stratum_core::config::{PreviewConfig, StratumConfig};

const DESCRIPTOR: &str = include_str!("../descriptor.hcl");
const KEY_S: u32 = 0x53;
const KEY_ESCAPE: u32 = 0x1B;
const KEY_LEFT: u32 = 0x25;
const KEY_UP: u32 = 0x26;
const KEY_RIGHT: u32 = 0x27;
const KEY_DOWN: u32 = 0x28;
const KEY_F10: u32 = 0x79;

// The checker with the manager's own key and field budgets for one mod.
struct Checker;

impl Checker {
    fn manager() -> DescriptorChecker {
        DescriptorChecker::new(128, 256)
    }
}

#[test]
fn descriptor_passes_the_manager_checker() {
    let descriptor = Checker::manager().parse(DESCRIPTOR).expect("the descriptor parses and checks");

    assert_eq!(
        (
            descriptor.namespace.as_str(),
            descriptor.name.as_str(),
            descriptor.display_name.as_str(),
            descriptor.version.as_str(),
            descriptor.author.as_str(),
            descriptor.config_path.as_str()
        ),
        ("alaydriem", "stratum", "Stratum", "0.1.0", "Alaydriem", "config.hcl")
    );
    assert_eq!(descriptor.input_passthrough, [KEY_ESCAPE]);
    let claims: Vec<_> =
        descriptor.claims.iter().map(|claim| (claim.capability, claim.mode, claim.block_player_turn)).collect();
    assert_eq!(
        claims,
        [
            (Capability::Overlay, ClaimMode::Shared, false),
            (Capability::WorldDraw, ClaimMode::Shared, false),
            (Capability::Hud, ClaimMode::Shared, false),
            (Capability::Input, ClaimMode::Exclusive, false),
        ]
    );
    let keybinds: Vec<_> = descriptor
        .keybinds
        .iter()
        .map(|keybind| {
            (
                keybind.action.as_str(),
                keybind.capability,
                keybind.scope,
                keybind.default_chord,
                keybind.config_path.as_str(),
            )
        })
        .collect();
    let f3_s = Chord { key: KEY_S, ctrl: false, shift: false, alt: false, f3: true };
    let held = |key: u32| Chord { key, ctrl: false, shift: false, alt: false, f3: false };
    assert_eq!(
        keybinds,
        [
            ("open_library", Capability::Overlay, KeyScope::Global, f3_s, "keys.open_library"),
            ("hud_up", Capability::Hud, KeyScope::WhileHeld, held(KEY_UP), "keys.hud_up"),
            ("hud_down", Capability::Hud, KeyScope::WhileHeld, held(KEY_DOWN), "keys.hud_down"),
            ("hud_left", Capability::Hud, KeyScope::WhileHeld, held(KEY_LEFT), "keys.hud_left"),
            ("hud_right", Capability::Hud, KeyScope::WhileHeld, held(KEY_RIGHT), "keys.hud_right"),
            ("hud_mode", Capability::Hud, KeyScope::WhileHeld, held(KEY_F10), "keys.hud_mode"),
        ]
    );
    let sections: Vec<_> = descriptor
        .settings
        .iter()
        .map(|section| {
            (section.id.as_str(), section.fields.iter().map(|field| field.path.as_str()).collect::<Vec<_>>())
        })
        .collect();
    assert_eq!(
        sections,
        [
            (
                "keys",
                vec![
                    "keys.open_library",
                    "keys.hud_up",
                    "keys.hud_down",
                    "keys.hud_left",
                    "keys.hud_right",
                    "keys.hud_mode",
                ],
            ),
            ("library", vec!["library.max_structures", "library.max_projects", "markers.max_per_world"]),
            ("import", vec!["import.max_file_mb", "import.inbox_poll_seconds"]),
            ("memory", vec!["cache.budget_mb", "preview.texture_cache", "preview.max_quads"]),
            ("ghost", vec!["ghost.miniature", "ghost.draw_distance", "ghost.mesh_budget_mb", "ghost.cells_per_frame"]),
            ("debug", vec!["debug.telemetry"]),
        ]
    );
}

#[test]
fn schema_defaults_match_core_defaults() {
    let descriptor = Checker::manager().parse(DESCRIPTOR).expect("the descriptor parses and checks");

    let checked = SchemaChecker::check(&descriptor.settings, HclDocument::empty());
    let config: StratumConfig = hcl::from_str(&checked.document.text()).expect("the default document parses");

    assert_eq!(config, StratumConfig::default());
}

#[test]
fn open_library_is_keybind_index_0() {
    let descriptor = Checker::manager().parse(DESCRIPTOR).expect("the descriptor parses and checks");

    let index = descriptor.keybinds.iter().position(|keybind| keybind.action == "open_library");

    assert_eq!(index, Some(usize::from(ManagerSession::OPEN_LIBRARY_ACTION)));
}

#[test]
fn only_the_key_telemetry_preview_and_ghost_settings_apply_live() {
    let descriptor = Checker::manager().parse(DESCRIPTOR).expect("the descriptor parses and checks");

    let live: Vec<_> = descriptor
        .settings
        .iter()
        .flat_map(|section| section.fields.iter())
        .filter(|field| field.apply != ApplyMode::Restart)
        .map(|field| (field.path.as_str(), field.apply))
        .collect();

    assert_eq!(
        live,
        [
            ("keys.open_library", ApplyMode::Live),
            ("keys.hud_up", ApplyMode::Live),
            ("keys.hud_down", ApplyMode::Live),
            ("keys.hud_left", ApplyMode::Live),
            ("keys.hud_right", ApplyMode::Live),
            ("keys.hud_mode", ApplyMode::Live),
            ("preview.texture_cache", ApplyMode::Live),
            ("preview.max_quads", ApplyMode::Live),
            ("ghost.miniature", ApplyMode::Live),
            ("ghost.draw_distance", ApplyMode::Live),
            ("ghost.cells_per_frame", ApplyMode::Live),
            ("debug.telemetry", ApplyMode::Live),
        ]
    );
}

#[test]
fn ghost_mesh_budget_applies_on_restart() {
    let descriptor = Checker::manager().parse(DESCRIPTOR).expect("the descriptor parses and checks");

    let apply = descriptor
        .settings
        .iter()
        .flat_map(|section| section.fields.iter())
        .find(|field| field.path == "ghost.mesh_budget_mb")
        .map(|field| field.apply);

    assert_eq!(apply, Some(ApplyMode::Restart));
}

#[test]
fn hud_keybinds_are_the_session_indexes_and_share_no_chord() {
    let descriptor = Checker::manager().parse(DESCRIPTOR).expect("the descriptor parses and checks");

    let index = |action: &str| descriptor.keybinds.iter().position(|keybind| keybind.action == action);

    assert_eq!(index("hud_up"), Some(usize::from(ManagerSession::HUD_UP_ACTION)));
    assert_eq!(index("hud_down"), Some(usize::from(ManagerSession::HUD_DOWN_ACTION)));
    assert_eq!(index("hud_left"), Some(usize::from(ManagerSession::HUD_LEFT_ACTION)));
    assert_eq!(index("hud_right"), Some(usize::from(ManagerSession::HUD_RIGHT_ACTION)));
    assert_eq!(index("hud_mode"), Some(usize::from(ManagerSession::HUD_MODE_ACTION)));
    for (position, keybind) in descriptor.keybinds.iter().enumerate() {
        for other in &descriptor.keybinds[position + 1..] {
            assert_ne!(keybind.default_chord, other.default_chord, "{} and {}", keybind.action, other.action);
        }
    }
}

#[test]
fn a_preview_bound_in_the_config_file_is_kept_and_held_to_its_range() {
    let descriptor = Checker::manager().parse(DESCRIPTOR).expect("the descriptor parses and checks");
    let text = "preview {\n  texture_cache = 64\n  max_quads = 9000000\n}\n";
    let document = HclDocument::new(text).expect("the document parses");

    let checked = SchemaChecker::check(&descriptor.settings, document);
    let config: StratumConfig = hcl::from_str(&checked.document.text()).expect("the checked document parses");

    assert_eq!(config.preview.texture_cache, 64);
    assert_eq!(config.preview.max_quads, PreviewConfig::MAX_MAX_QUADS);
}
