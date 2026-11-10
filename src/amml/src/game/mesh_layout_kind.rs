use amml_game::GameEntries;

use crate::game::{MeshLayout2651, MeshLayout2660};
use crate::types::StratumEntry;

/// Which `mce::Mesh` mirror the game build has, from the value of the `Mesh.size` entry; a size that matches no
/// mirror selects none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshLayoutKind {
    /// [`MeshLayout2651`].
    Build2651,
    /// [`MeshLayout2660`].
    Build2660,
}

impl MeshLayoutKind {
    /// The mirror whose size is the entry value; `None` when the entry is off or no mirror has it.
    pub fn of(entries: &GameEntries) -> Option<Self> {
        match entries.get(StratumEntry::MESH_SIZE)? {
            MeshLayout2651::SIZE => Some(Self::Build2651),
            MeshLayout2660::SIZE => Some(Self::Build2660),
            _ => None,
        }
    }
}
