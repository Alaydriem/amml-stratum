use amml_i18n::text::{Args, Text, UserText};
use stratum_core::structs::material::StackCount;

use super::DigitGroups;
use crate::message::StratumMessage;

// The words for a count in stacks: "53 stacks + 20", "24 stacks" or "42".
pub struct StackText;

impl StackText {
    pub fn of(stacks: StackCount) -> Text {
        let rest = Self::number("rest", stacks.rest);
        if stacks.stacks == 0 {
            return Text::format(StratumMessage::RestOnly, rest);
        }
        let args = Self::number("count", stacks.stacks);
        if stacks.rest == 0 {
            Text::plural(StratumMessage::StacksOnly, stacks.stacks, args)
        } else {
            let args = args.user("rest", UserText::from_model(DigitGroups::of(stacks.rest)));
            Text::plural(StratumMessage::StacksAndRest, stacks.stacks, args)
        }
    }

    fn number(name: &'static str, value: u64) -> Args {
        Args::new().user(name, UserText::from_model(DigitGroups::of(value)))
    }
}
