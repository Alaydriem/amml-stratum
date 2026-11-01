use egui::Context;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::snapshot::LibrarySnapshot;

use crate::views::dialogs::{AssignDialog, DeleteDialog, DialogOutcome, MoveDialog, NewProjectDialog, RenameDialog};

// The one question or form the menu has open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    NewProject(NewProjectDialog),
    Rename(RenameDialog),
    Move(MoveDialog),
    Delete(DeleteDialog),
    Assign(AssignDialog),
}

impl Dialog {
    // Draws the open dialog. It pushes its action when the user confirms.
    pub fn show(&mut self, ctx: &Context, snapshot: &LibrarySnapshot, out: &mut Vec<LibraryAction>) -> DialogOutcome {
        match self {
            Self::NewProject(dialog) => dialog.show(ctx, snapshot, out),
            Self::Rename(dialog) => dialog.show(ctx, snapshot, out),
            Self::Move(dialog) => dialog.show(ctx, snapshot, out),
            Self::Delete(dialog) => dialog.show(ctx, snapshot, out),
            Self::Assign(dialog) => dialog.show(ctx, snapshot, out),
        }
    }
}
