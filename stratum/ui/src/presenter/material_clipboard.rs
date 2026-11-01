use amml_i18n::text::{Args, Text, UserText};

use super::{DigitGroups, MaterialRow, StackText};
use crate::message::StratumMessage;

// Writes the rows of the materials table as the text the copy button puts on the clipboard.
pub struct MaterialClipboard;

impl MaterialClipboard {
    // The most bytes the Mod Manager clipboard bridge takes.
    pub const MAX_BYTES: usize = 1_048_576;

    // The heading, then one line per row. Every line ends with CR LF, which Windows expects.
    pub fn render(rows: &[MaterialRow], heading: &str, localize: impl Fn(&Text) -> String) -> String {
        let mut out = String::with_capacity(heading.len() + 2 + rows.len() * 40);
        out.push_str(heading);
        out.push_str("\r\n");
        for row in rows {
            let stacks = localize(&StackText::of(row.stacks));
            let args = Args::new()
                .user("item", UserText::from_model(row.name.clone()))
                .user("count", UserText::from_model(DigitGroups::of(row.count)))
                .user("stacks", UserText::from_model(stacks));
            out.push_str(&localize(&Text::format(StratumMessage::ClipboardLine, args)));
            out.push_str("\r\n");
        }
        out
    }

    pub fn fits(text: &str) -> bool {
        text.len() <= Self::MAX_BYTES
    }
}
