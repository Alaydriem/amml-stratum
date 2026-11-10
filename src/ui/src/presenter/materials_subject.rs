use amml_i18n::text::{Args, Text, UserText};
use stratum_core::structs::action::MaterialScope;
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::DigitGroups;
use crate::message::StratumMessage;

// What a materials tab lists: its scope, the name that heads the list and the totals under the heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialsSubject {
    pub scope: MaterialScope,
    pub name: String,
    pub structures: usize,
    pub blocks: u64,
}

impl MaterialsSubject {
    // The subject of `scope`, or None when the snapshot no longer has it.
    pub fn of(snapshot: &LibrarySnapshot, scope: MaterialScope) -> Option<Self> {
        match scope {
            MaterialScope::Project(id) => {
                let project = snapshot.projects.iter().find(|project| project.id == id)?;
                let blocks = project.structures.iter().fold(0u64, |sum, row| sum.saturating_add(row.block_count));
                Some(Self { scope, name: project.name.clone(), structures: project.structures.len(), blocks })
            }
            MaterialScope::Structure(id) => {
                let row = snapshot
                    .unsorted
                    .iter()
                    .chain(snapshot.projects.iter().flat_map(|project| project.structures.iter()))
                    .find(|row| row.id == id)?;
                Some(Self { scope, name: row.name.clone(), structures: 1, blocks: row.block_count })
            }
        }
    }

    // "All 4 structures" for a project, "Main hall only" for a structure.
    pub fn heading(&self) -> Text {
        match self.scope {
            MaterialScope::Project(_) => Text::plural(
                StratumMessage::MaterialsAllStructures,
                u64::try_from(self.structures).unwrap_or(u64::MAX),
                Args::new(),
            ),
            MaterialScope::Structure(_) => Text::format(
                StratumMessage::MaterialsOnly,
                Args::new().user("name", UserText::from_model(self.name.clone())),
            ),
        }
    }

    // "38 items · 11,904 blocks · sorted by count".
    pub fn summary(&self, items: usize) -> Text {
        let count = u64::try_from(items).unwrap_or(u64::MAX);
        let items = DigitGroups::of(count);
        let args = Args::new()
            .user("items", UserText::from_model(items))
            .user("blocks", UserText::from_model(DigitGroups::of(self.blocks)));
        Text::plural(StratumMessage::MaterialsSummary, count, args)
    }

    pub fn copy_label(&self) -> Text {
        match self.scope {
            MaterialScope::Project(_) => Text::message(StratumMessage::CopyProjectList),
            MaterialScope::Structure(_) => Text::message(StratumMessage::CopyStructureList),
        }
    }
}
