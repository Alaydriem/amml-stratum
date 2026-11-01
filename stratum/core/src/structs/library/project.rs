use super::{ProjectId, ProjectKind};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub kind: ProjectKind,
}
