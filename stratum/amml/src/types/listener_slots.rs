/// The resolved vtable slots the block listener uses: two on `BlockSource`, two on `BlockSourceListener`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListenerSlots {
    pub add_listener: u64,
    pub remove_listener: u64,
    pub source_destroyed: u64,
    pub block_changed: u64,
}
