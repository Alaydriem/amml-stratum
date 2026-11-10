use amml_i18n::text::{Args, Text};
use stratum_core::structs::library::NameLimit;

use crate::message::StratumMessage;

// Tells whether a name typed in a dialog can be sent to the library, and why not.
pub struct NameCheck;

impl NameCheck {
    // The reason `name` cannot be used, or None. The name is trimmed first and counted in characters against the
    // core limit, as core counts it.
    pub fn reason(name: &str) -> Option<Text> {
        let count = name.trim().chars().count();
        if count == 0 {
            Some(Text::message(StratumMessage::ReasonNameEmpty))
        } else if count > NameLimit::MAX_CHARS {
            Some(Self::too_long())
        } else {
            None
        }
    }

    // "The name is longer than {max} characters." with the core limit filled in.
    pub fn too_long() -> Text {
        let max = u64::try_from(NameLimit::MAX_CHARS).unwrap_or(u64::MAX);
        Text::format(StratumMessage::ReasonNameTooLong, Args::new().uint("max", max))
    }
}
