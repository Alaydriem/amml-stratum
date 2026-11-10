use crate::structs::action::HudAction;
use crate::structs::library::StructureId;
use crate::structs::marker::{Marker, ViewMode};

// Applies a HUD action to one marker.
pub struct MarkerControls;

impl MarkerControls {
    // `layers` is the layer count of the build that is shown. `order` is the structures a normal
    // project cycles through, by name, and empty for every other target.
    pub fn apply(marker: &mut Marker, action: HudAction, layers: u32, order: &[StructureId]) {
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
            HudAction::StructureNext => Self::step_structure(marker, order, true),
            HudAction::StructurePrev => Self::step_structure(marker, order, false),
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

    // One step through `order`, wrapping. Nothing happens with fewer than two structures or when the
    // marker shows none of them.
    fn step_structure(marker: &mut Marker, order: &[StructureId], forward: bool) {
        if order.len() < 2 {
            return;
        }
        let Some(at) = marker.view.current.and_then(|current| order.iter().position(|id| *id == current)) else {
            return;
        };
        let next = if forward { (at + 1) % order.len() } else { (at + order.len() - 1) % order.len() };
        marker.view.switch_to(order[next]);
    }
}
