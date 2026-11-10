pub mod assign_choice;
pub mod assign_dialog;
pub mod assign_subject;
pub mod delete_dialog;
pub mod dialog_form;
pub mod move_dialog;
pub mod new_project_dialog;
pub mod rename_dialog;

pub use assign_choice::AssignChoice;
pub use assign_dialog::AssignDialog;
pub use assign_subject::AssignSubject;
pub use delete_dialog::DeleteDialog;
pub use dialog_form::DialogForm;
pub use move_dialog::MoveDialog;
pub use new_project_dialog::NewProjectDialog;
pub use rename_dialog::RenameDialog;

// Whether a dialog stays on the screen after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogOutcome {
    Open,
    Closed,
}

// What the user did in a dialog frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormAnswer {
    Pending,
    Confirm,
    Cancel,
}
