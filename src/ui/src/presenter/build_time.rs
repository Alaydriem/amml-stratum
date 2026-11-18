use amml_i18n::text::{Args, Text};

use crate::message::StratumMessage;

// The words for a time of a build: "45 s", "12 min 30 s" or "2 h 5 min". `estimate` is the plain time; `left` adds
// "Time left" in front of it.
pub struct BuildTime;

impl BuildTime {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 3600;

    // The time the build takes, as the settings panel shows it.
    pub fn estimate(seconds: u64) -> Text {
        Self::words(
            seconds,
            StratumMessage::BuildTimeSeconds,
            StratumMessage::BuildTimeMinutes,
            StratumMessage::BuildTimeHours,
        )
    }

    // The time the build still needs, as the HUD card chip shows it.
    pub fn left(seconds: u64) -> Text {
        Self::words(
            seconds,
            StratumMessage::BuildTimeLeftSeconds,
            StratumMessage::BuildTimeLeftMinutes,
            StratumMessage::BuildTimeLeftHours,
        )
    }

    // The whole seconds a build of `blocks` takes at `rate` blocks a second, rounded up. A rate of 0 counts as 1.
    pub fn seconds(blocks: u64, rate: u64) -> u64 {
        blocks.div_ceil(rate.max(1))
    }

    fn words(seconds: u64, short: StratumMessage, minutes: StratumMessage, hours: StratumMessage) -> Text {
        if seconds < Self::MINUTE {
            return Text::format(short, Args::new().uint("seconds", seconds));
        }
        if seconds < Self::HOUR {
            let args = Args::new().uint("minutes", seconds / Self::MINUTE).uint("seconds", seconds % Self::MINUTE);
            return Text::format(minutes, args);
        }
        let args = Args::new().uint("hours", seconds / Self::HOUR).uint("minutes", seconds % Self::HOUR / Self::MINUTE);
        Text::format(hours, args)
    }
}
