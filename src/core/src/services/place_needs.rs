use crate::services::ItemRules;
use crate::structs::easy_place::{FaceRule, Half, PlaceNeed};
use crate::structs::ghost::GhostPalette;
use crate::structs::material::ItemYield;
use crate::structs::structure::{BlockState, StateValue};

const NOT_A_TARGET: PlaceNeed = PlaceNeed::NotTarget;

// The placement need of every palette entry, built once per build.
#[derive(Debug, Clone)]
pub struct PlaceNeeds {
    needs: Vec<PlaceNeed>,
    names: Vec<String>,
}

impl PlaceNeeds {
    // Blocks that are never targets.
    const LIQUIDS: &[&str] = &["water", "flowing_water", "lava", "flowing_lava"];

    const WALL_NAMES: &[&str] = &["wall_torch", "wall_sign", "wall_banner", "ladder"];
    const WALL_SUFFIXES: &[&str] = &["_wall_torch", "_wall_sign", "_wall_hanging_sign", "_wall_fan"];

    const FLOOR_NAMES: &[&str] =
        &["torch", "soul_torch", "redstone_torch", "unlit_redstone_torch", "standing_sign", "standing_banner"];
    const FLOOR_SUFFIXES: &[&str] = &["_standing_sign"];

    pub fn new(palette: &GhostPalette, rules: &ItemRules) -> Self {
        let marker = GhostPalette::marker_entry();
        let mut needs = Vec::with_capacity(palette.entries.len());
        let mut names = Vec::with_capacity(palette.entries.len());
        for block in &palette.entries {
            names.push(block.name.clone());
            let short = block.name.strip_prefix(ItemRules::NAMESPACE).unwrap_or(&block.name);
            if *block == marker || Self::LIQUIDS.contains(&short) {
                needs.push(PlaceNeed::NotTarget);
                continue;
            }
            let (item, slab_of) = match rules.items_for(block) {
                ItemYield::None => {
                    needs.push(PlaceNeed::NotTarget);
                    continue;
                }
                ItemYield::One(item, count) => {
                    let slab_of = (count == 2 && short.ends_with("_double_slab")).then(|| item.0.clone());
                    (item, slab_of)
                }
                ItemYield::Two(first, ..) => (first, None),
            };
            needs.push(PlaceNeed::Place {
                item,
                rule: Self::rule_for(short),
                slab_of,
                half: Self::half_of(short, block),
            });
        }
        Self { needs, names }
    }

    // An unknown id is not a target.
    pub fn need(&self, palette_id: u16) -> &PlaceNeed {
        self.needs.get(usize::from(palette_id)).unwrap_or(&NOT_A_TARGET)
    }

    pub fn block_name(&self, palette_id: u16) -> Option<&str> {
        self.names.get(usize::from(palette_id)).map(String::as_str)
    }

    // The half a slab or stair fills. A double slab fills both halves, so it is `Any` even when the structure gives it
    // a `minecraft:vertical_half` state.
    fn half_of(short: &str, block: &BlockState) -> Half {
        if short.ends_with("_double_slab") {
            return Half::Any;
        }
        if let Some(StateValue::Text(text)) = block.states.get("minecraft:vertical_half") {
            return match text.as_str() {
                "top" => Half::Top,
                "bottom" => Half::Bottom,
                _ => Half::Any,
            };
        }
        let flag = match block.states.get("upside_down_bit") {
            Some(StateValue::Bool(value)) => *value,
            Some(StateValue::Int(value)) => *value != 0,
            _ => return Half::Any,
        };
        if flag { Half::Top } else { Half::Bottom }
    }

    fn rule_for(short: &str) -> FaceRule {
        if Self::WALL_NAMES.contains(&short) || Self::WALL_SUFFIXES.iter().any(|suffix| short.ends_with(suffix)) {
            FaceRule::Wall
        } else if Self::FLOOR_NAMES.contains(&short)
            || Self::FLOOR_SUFFIXES.iter().any(|suffix| short.ends_with(suffix))
        {
            FaceRule::Floor
        } else {
            FaceRule::Any
        }
    }
}
