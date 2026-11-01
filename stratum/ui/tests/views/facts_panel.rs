use amml_ui::tokens::{ColorToken, color};
use egui::Rect;
use stratum_core::structs::library::StructureId;
use stratum_ui::state::Page;

use crate::support::{ShapeProbe, SnapshotFixture, ViewSession};

// The card of the structure "Main hall" (id 1, 32 x 18 x 24, 4,212 blocks) on its own page.
struct Card;

impl Card {
    fn page(session: &mut ViewSession) {
        session.state.page = Page::Structure(StructureId(1));
        session.frame(Vec::new());
        session.frame(Vec::new());
    }

    fn session() -> ViewSession {
        let mut session = ViewSession::new(SnapshotFixture::library());
        Self::page(&mut session);
        session
    }

    fn one(session: &ViewSession, text: &str) -> Rect {
        let rects = ShapeProbe::exact_rects(&session.shapes, text);
        assert_eq!(rects.len(), 1, "{text} is painted once: {rects:?}");
        rects[0]
    }
}

#[test]
fn the_facts_are_one_line_of_size_layers_blocks_and_origin() {
    let session = Card::session();
    let parts = ["32 × 18 × 24", "18 layers", "4,212 blocks", "no saved origin"].map(|text| Card::one(&session, text));
    for pair in parts.windows(2) {
        assert!(pair[0].right() < pair[1].left(), "the parts run left to right: {pair:?}");
        assert!((pair[0].center().y - pair[1].center().y).abs() < 1.0, "the parts share one line: {pair:?}");
    }
}

#[test]
fn the_facts_header_the_project_row_and_the_name_row_are_gone() {
    let session = Card::session();
    for text in ["STRUCTURE FACTS", "Project", "Size", "Layers", "Blocks", "Saved origin", "Name"] {
        assert!(ShapeProbe::exact_rects(&session.shapes, text).is_empty(), "{text} is painted");
    }
}

#[test]
fn a_saved_origin_is_named_with_its_three_numbers() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.projects[0].structures[0].origin = Some([112, 64, -340]);
    let mut session = ViewSession::new(snapshot);
    Card::page(&mut session);
    let size = Card::one(&session, "32 × 18 × 24");
    let origin = Card::one(&session, "origin 112, 64, -340");
    assert!((origin.center().y - size.center().y).abs() < 1.0);
}

#[test]
fn without_a_preview_the_size_box_names_the_three_lengths() {
    let session = Card::session();
    for length in ["32", "18", "24"] {
        assert_eq!(ShapeProbe::exact_rects(&session.shapes, length).len(), 1, "{length} is in the size box");
    }
}

#[test]
fn the_preview_fills_the_free_height_of_the_card() {
    let mut session = ViewSession::with_preview(SnapshotFixture::library());
    Card::page(&mut session);
    let facts = Card::one(&session, "32 × 18 × 24");
    let delete = Card::one(&session, "Delete");
    let footer = Card::one(&session, "Place marker here");
    // Paper: the actions sit 20 above the footer divider, the footer button 16 under it; both labels are 18 high
    // in 36 high buttons
    let gap = footer.top() - delete.bottom();
    assert!((50.0..=60.0).contains(&gap), "the actions end {gap} above the footer");
    let hint = Card::one(&session, "Drag to turn");
    let below = facts.top() - hint.bottom();
    assert!((20.0..=30.0).contains(&below), "the hint ends {below} above the facts line");
    assert!(ShapeProbe::exact_rects(&session.shapes, "32").is_empty(), "the size box is not drawn");
}

#[test]
fn the_hint_names_drag_then_scroll_at_the_bottom_left_of_the_pane() {
    let mut session = ViewSession::with_preview(SnapshotFixture::library());
    Card::page(&mut session);
    let drag = Card::one(&session, "Drag to turn");
    let zoom = Card::one(&session, "Scroll to zoom");
    let facts = Card::one(&session, "32 × 18 × 24");
    assert!(drag.right() < zoom.left());
    assert!((drag.center().y - zoom.center().y).abs() < 1.0);
    // Paper: the hint is 12 in from the pane's left edge, and the facts line starts at that edge
    let inset = drag.left() - facts.left();
    assert!((11.0..=13.0).contains(&inset), "the hint is {inset} in from the pane's edge");
}

#[test]
fn the_render_time_shows_in_a_pill_at_the_top_right_of_the_pane() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.projects[0].structures[0].file_name = "tint.mcstructure".to_owned();
    let samples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples");
    let mut session = ViewSession::with_preview_files(snapshot, samples);
    Card::page(&mut session);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let time = loop {
        session.frame(Vec::new());
        if let Some(rect) = ShapeProbe::text_rect(&session.shapes, " ms") {
            break rect;
        }
        assert!(std::time::Instant::now() < deadline, "no picture arrived");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let pill_fill = <color::Pill as ColorToken>::VALUE;
    let pill = ShapeProbe::fill_rects(&session.shapes, pill_fill)
        .into_iter()
        .find(|rect| rect.contains_rect(time))
        .expect("the time sits on a pill");
    let field = <color::BgField as ColorToken>::VALUE;
    let pane = ShapeProbe::fill_rects(&session.shapes, field)
        .into_iter()
        .find(|rect| rect.contains_rect(pill))
        .expect("the pill is inside the pane");
    // Paper: the pill is 10 in from the pane's top and right edges, with 8 x 2 padding round the text
    assert!((pane.right() - pill.right() - 10.0).abs() <= 1.0, "pill {pill:?} pane {pane:?}");
    assert!((pill.top() - pane.top() - 10.0).abs() <= 1.0, "pill {pill:?} pane {pane:?}");
    assert!((time.left() - pill.left() - 8.0).abs() <= 1.0 && (time.top() - pill.top() - 2.0).abs() <= 1.0);
}
