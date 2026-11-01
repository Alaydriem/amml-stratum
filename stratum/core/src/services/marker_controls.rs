use crate::structs::action::HudAction;
use crate::structs::marker::{Marker, ViewMode};

// Applies a HUD action to one marker.
pub struct MarkerControls;

impl MarkerControls {
    // `layers` is the layer count of the build that is shown. `structures` is the count a normal
    // project cycles through, and 1 for every other target.
    pub fn apply(marker: &mut Marker, action: HudAction, layers: u32, structures: u32) {
        match action {
            HudAction::LayerUp => Self::layer_up(marker, layers),
            HudAction::LayerDown => Self::layer_down(marker, layers),
            HudAction::ToggleShown => marker.view.shown = !marker.view.shown,
            HudAction::OpacityUp => marker.view.opacity_step = marker.view.opacity().up().index(),
            HudAction::OpacityDown => marker.view.opacity_step = marker.view.opacity().down().index(),
            HudAction::Move(axis, direction) => {
                let slot = &mut marker.origin[axis.index()];
                *slot = slot.saturating_add(i32::from(direction.signum()));
            }
            HudAction::Rotate(quarters) => marker.rotation = marker.rotation.turned(quarters),
            HudAction::ToggleEasyPlace => marker.view.easy_place = !marker.view.easy_place,
            HudAction::StructureNext => Self::step_structure(marker, structures, true),
            HudAction::StructurePrev => Self::step_structure(marker, structures, false),
        }
    }

    // Bottom to top, then the whole build, then the bottom again.
    fn layer_up(marker: &mut Marker, layers: u32) {
        if layers == 0 {
            return;
        }
        match marker.view.shown_layer(layers) {
            None => Self::show_layer(marker, 0),
            Some(layer) if layer + 1 < layers => Self::show_layer(marker, layer + 1),
            Some(_) => marker.view.view_mode = ViewMode::Whole,
        }
    }

    // Top to bottom, then the whole build, then the top again.
    fn layer_down(marker: &mut Marker, layers: u32) {
        if layers == 0 {
            return;
        }
        match marker.view.shown_layer(layers) {
            None => Self::show_layer(marker, layers - 1),
            Some(0) => marker.view.view_mode = ViewMode::Whole,
            Some(layer) => Self::show_layer(marker, layer - 1),
        }
    }

    fn show_layer(marker: &mut Marker, layer: u32) {
        marker.view.view_mode = ViewMode::Layer;
        marker.view.layer = layer;
    }

    fn step_structure(marker: &mut Marker, structures: u32, forward: bool) {
        let count = structures.max(1);
        let current = marker.view.selected.min(count - 1);
        marker.view.selected = if forward { (current + 1) % count } else { (current + count - 1) % count };
    }
}
