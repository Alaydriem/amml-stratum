// Identity of one world and dimension. The world name is any text the game gives.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WorldKey {
    pub world: String,
    pub dimension: i32,
}

impl WorldKey {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    // Lowercase hex FNV-1a 64 hash of the world name, `-`, then the dimension. Only `[0-9a-f-]`
    // appear, so any world name gives a safe file name.
    pub fn file_stem(&self) -> String {
        let hash = self
            .world
            .as_bytes()
            .iter()
            .fold(Self::FNV_OFFSET, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(Self::FNV_PRIME));
        format!("{hash:016x}-{}", self.dimension)
    }
}
