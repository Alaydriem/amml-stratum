use amml_ui::tokens::{SpacingToken, spacing};
use amml_ui::widget::ToastLine;
use egui::Ui;
use stratum_core::structs::snapshot::LibrarySnapshot;

use crate::presenter::NoticeText;

// One toast line for each of the newest notices of the snapshot, newest first: import failures, refusals, a busy
// structure reader and the rest. Older notices are not shown, so the page under them keeps its room.
pub struct ImportNotices;

impl ImportNotices {
    pub const SHOWN: usize = 3;

    pub fn show(ui: &mut Ui, snapshot: &LibrarySnapshot) {
        for notice in snapshot.notices.iter().rev().take(Self::SHOWN) {
            let (tone, text) = NoticeText::of(notice);
            ToastLine::new(tone, text).show(ui);
            ui.add_space(<spacing::S as SpacingToken>::VALUE);
        }
    }
}
