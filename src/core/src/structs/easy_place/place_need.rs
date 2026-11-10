use crate::structs::easy_place::{FaceRule, Half};
use crate::structs::material::ItemId;

// What easy place must do for one palette entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceNeed {
    NotTarget,
    Place {
        item: ItemId,
        rule: FaceRule,
        // For a double slab, the single slab block name that also counts as not yet filled.
        slab_of: Option<String>,
        half: Half,
    },
}
