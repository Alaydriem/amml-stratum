// The file name follows the type, as the layout rules require.
#[allow(clippy::module_inception)]
pub mod library;
pub mod name_limit;
pub mod new_structure;
pub mod project;
pub mod project_id;
pub mod project_kind;
pub mod project_order;
pub mod structure_entry;
pub mod structure_id;

pub use library::Library;
pub use name_limit::NameLimit;
pub use new_structure::NewStructure;
pub use project::Project;
pub use project_id::ProjectId;
pub use project_kind::ProjectKind;
pub use project_order::ProjectOrder;
pub use structure_entry::StructureEntry;
pub use structure_id::StructureId;
