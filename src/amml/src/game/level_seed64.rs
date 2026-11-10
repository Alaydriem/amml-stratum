/// The game's `LevelSeed64`: one `uint64`, returned through a hidden pointer by a member function.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LevelSeed64 {
    pub value: u64,
}

const _: () = assert!(size_of::<LevelSeed64>() == 8);
