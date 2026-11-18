use std::fmt::Write;

use crate::structs::build::{ClearPart, PaletteOutcome};
use crate::structs::structure::{BlockState, StateValue};

// Writes the chat commands the build sends.
pub struct CommandText;

impl CommandText {
    // A setblock command. `NameOnly` drops the state list. `Unknown` reads like `Exact`: the
    // build never asks for text for a block outside the palette.
    pub fn set_block(position: [i32; 3], block: &BlockState, outcome: PaletteOutcome) -> String {
        let [x, y, z] = position;
        let mut text = String::with_capacity(48 + block.name.len());
        let _ = write!(text, "/setblock {x} {y} {z} {}", block.name);
        if outcome == PaletteOutcome::NameOnly || block.states.is_empty() {
            return text;
        }
        text.push_str(" [");
        for (index, (key, value)) in block.states.iter().enumerate() {
            if index > 0 {
                text.push(',');
            }
            let _ = write!(text, "\"{key}\"=");
            match value {
                StateValue::Bool(flag) => {
                    let _ = write!(text, "{flag}");
                }
                StateValue::Int(number) => {
                    let _ = write!(text, "{number}");
                }
                StateValue::Text(label) => Self::push_quoted(&mut text, label),
            }
        }
        text.push(']');
        text
    }

    // A fill command that turns the part into air, limited to one block kind when the part has one.
    pub fn fill_air(part: &ClearPart) -> String {
        let [x1, y1, z1] = part.min;
        let [x2, y2, z2] = part.max;
        let mut text = String::with_capacity(48);
        let _ = write!(text, "/fill {x1} {y1} {z1} {x2} {y2} {z2} air");
        if let Some(kind) = &part.kind {
            let _ = write!(text, " replace {kind}");
        }
        text
    }

    pub fn game_rule(name: &str, value: bool) -> String {
        format!("/gamerule {name} {value}")
    }

    // A quoted text value with backslash and quote escaped.
    fn push_quoted(text: &mut String, label: &str) {
        text.push('"');
        for character in label.chars() {
            if character == '\\' || character == '"' {
                text.push('\\');
            }
            text.push(character);
        }
        text.push('"');
    }
}
