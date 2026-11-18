use stratum_core::structs::build::BuildConditions;

use crate::traits::BuildFacts;
use crate::types::ConnectionInfo;

/// Reads the four conditions that say a build may run. With the feature off, or with no live local player in a
/// loaded world, every condition is false and no game call is made. The connection type of a world does not change
/// while the world stays, and reading it copies the game's connection info, so it is read once for each client level.
#[derive(Clone, Copy, Debug)]
pub struct ConditionsReader {
    feature_on: bool,
    // the client level the connection type was read for, and the type
    connection: Option<(usize, i16)>,
}

impl ConditionsReader {
    // `GameType::Creative`
    const CREATIVE: i32 = 1;
    // `PlayerPermissionLevel::Operator`; Custom (3) is above it
    const OPERATOR: u8 = 2;

    pub fn new(feature_on: bool) -> Self {
        Self { feature_on, connection: None }
    }

    pub fn read(&mut self, facts: &dyn BuildFacts) -> BuildConditions {
        if !self.feature_on || !facts.in_world() {
            // a level address can be reused by the next world, so nothing is kept out of a world
            self.connection = None;
            return BuildConditions::default();
        }
        BuildConditions {
            local: !facts.in_realm() && self.connection_type(facts) == Some(ConnectionInfo::LOCAL),
            cheats: facts.commands_flag().is_some_and(|flag| flag != 0),
            operator: facts.permission_level().is_some_and(|level| level >= Self::OPERATOR),
            creative: facts.game_type() == Some(Self::CREATIVE),
        }
    }

    // the connection type of the loaded world; a type the game has not set yet is read again next time
    fn connection_type(&mut self, facts: &dyn BuildFacts) -> Option<i16> {
        let Some(level) = facts.level() else {
            self.connection = None;
            return None;
        };
        if let Some((known, kind)) = self.connection
            && known == level
        {
            return Some(kind);
        }
        let kind = facts.connection_type()?;
        if kind != ConnectionInfo::UNDEFINED {
            self.connection = Some((level, kind));
        }
        Some(kind)
    }
}
