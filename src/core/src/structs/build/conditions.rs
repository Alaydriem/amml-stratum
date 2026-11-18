// What the game says about the world and the player, which decides if a build may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildConditions {
    // The world is a local world, not a server or a Realm.
    pub local: bool,
    // Commands work in the world.
    pub cheats: bool,
    pub operator: bool,
    pub creative: bool,
}

impl BuildConditions {
    pub fn allowed(&self) -> bool {
        self.local && self.cheats && self.operator && self.creative
    }
}
