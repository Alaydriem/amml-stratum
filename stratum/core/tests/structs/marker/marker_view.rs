use stratum_core::structs::library::StructureId;
use stratum_core::structs::marker::{MarkerView, ViewMode};

#[test]
fn switch_saves_and_restores_layer_per_structure() {
    let (a, b) = (StructureId(1), StructureId(2));
    let mut view = MarkerView::default();
    view.switch_to(a);
    view.layer = 5;
    view.view_mode = ViewMode::Layer;

    view.switch_to(b);
    assert_eq!((view.layer, view.view_mode), (0, ViewMode::Whole));
    view.layer = 2;
    view.view_mode = ViewMode::Layer;

    view.switch_to(a);
    assert_eq!((view.layer, view.view_mode), (5, ViewMode::Layer));
    assert_eq!(view.current, Some(a));
    view.switch_to(b);
    assert_eq!((view.layer, view.view_mode), (2, ViewMode::Layer));
}

#[test]
fn switch_to_current_changes_nothing() {
    let a = StructureId(1);
    let mut view = MarkerView::default();
    view.switch_to(a);
    view.layer = 3;
    view.view_mode = ViewMode::Layer;
    let before = view.clone();

    view.switch_to(a);
    assert_eq!(view, before);
}

#[test]
fn parts_are_bounded_and_evict_least_recent() {
    let mut view = MarkerView::default();
    let first = StructureId(0);
    view.switch_to(first);
    view.layer = 7;
    view.view_mode = ViewMode::Layer;
    for id in 1..=MarkerView::MAX_PARTS as u64 + 1 {
        view.switch_to(StructureId(id));
    }
    assert_eq!(view.parts.len(), MarkerView::MAX_PARTS);
    assert!(view.parts.iter().all(|part| part.structure != first));

    view.switch_to(first);
    assert_eq!((view.layer, view.view_mode), (0, ViewMode::Whole));
    assert!(view.parts.len() <= MarkerView::MAX_PARTS);
}

#[test]
fn keep_parts_drops_structures_not_kept() {
    let mut view = MarkerView::default();
    for id in 1..=3 {
        view.switch_to(StructureId(id));
    }
    assert_eq!(view.parts.len(), 2);

    view.keep_parts(&[StructureId(2)]);
    assert_eq!(view.parts.len(), 1);
    assert_eq!(view.parts[0].structure, StructureId(2));

    view.forget_parts();
    assert!(view.parts.is_empty());
}
