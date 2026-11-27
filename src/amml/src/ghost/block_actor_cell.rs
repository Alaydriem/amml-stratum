use crate::ghost::BlockActorKey;

/// One block-entity cell a slot's build kept: its ghost key and its position from the job's origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockActorCell {
    pub key: BlockActorKey,
    pub local: [i32; 3],
}

impl BlockActorCell {
    pub fn new(key: BlockActorKey, local: [i32; 3]) -> Self {
        Self { key, local }
    }
}
