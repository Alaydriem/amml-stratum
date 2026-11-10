use super::{ProjectId, ProjectKind};
use crate::structs::material::GatheredItem;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub kind: ProjectKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gathered: Vec<GatheredItem>,
}
