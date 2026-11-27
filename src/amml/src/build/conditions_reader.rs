use amml_api::{ConnectionKind, GameType, PermissionLevel};
use stratum_core::structs::build::BuildConditions;

use crate::traits::BuildFacts;

/// Reads the four conditions that say a build may run. With the feature off, or with no local player in a joined
/// world, every condition is false and no game call is made. The connection of a world does not change while the world
/// stays, so its kind is read once for each world epoch.
#[derive(Clone, Copy, Debug)]
pub struct ConditionsReader {
    feature_on: bool,
    // the world epoch the connection kind was read for, and the kind
    connection: Option<(u64, ConnectionKind)>,
}

impl ConditionsReader {
    pub fn new(feature_on: bool) -> Self {
        Self { feature_on, connection: None }
    }

    pub fn read(&mut self, facts: &dyn BuildFacts) -> BuildConditions {
        if !self.feature_on || !facts.in_world() {
            self.connection = None;
            return BuildConditions::default();
        }
        BuildConditions {
            local: !facts.in_realm() && self.connection_kind(facts) == Some(ConnectionKind::Local),
            cheats: facts.commands_enabled() == Some(true),
            operator: facts.permission_level().is_some_and(|level| level >= PermissionLevel::Operator),
            creative: facts.game_type() == Some(GameType::Creative),
        }
    }

    // the connection kind of the joined world; a world whose connection the game has not set yet, or set as undefined,
    // is read again next time
    fn connection_kind(&mut self, facts: &dyn BuildFacts) -> Option<ConnectionKind> {
        let Some(level) = facts.level() else {
            self.connection = None;
            return None;
        };
        if let Some((known, kind)) = self.connection
            && known == level
        {
            return Some(kind);
        }
        let kind = facts.connection_kind()?;
        // the game's undefined connection type comes as Other: it is read again until the game sets it
        if kind != ConnectionKind::Other {
            self.connection = Some((level, kind));
        }
        Some(kind)
    }
}
