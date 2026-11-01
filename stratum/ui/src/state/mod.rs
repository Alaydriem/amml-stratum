pub mod dialog;
pub mod dialog_item;
pub mod library_ui_state;
pub mod page;
pub mod pick_flash;

pub use dialog::Dialog;
pub use dialog_item::DialogItem;
pub use library_ui_state::LibraryUiState;
pub use page::Page;
pub use pick_flash::PickFlash;

// The tabs of a project page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTab {
    Structures,
    Materials,
}

// The tabs of a structure page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureTab {
    Facts,
    Materials,
}
