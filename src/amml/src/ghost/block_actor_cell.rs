use stratum_core::structs::ghost::SkullPose;

use crate::ghost::BlockActorKey;

/// One block-entity cell a slot's build kept: its actor key, its position from the job's origin and its skull pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockActorCell {
    pub key: BlockActorKey,
    pub local: [i32; 3],
    pub skull: Option<SkullPose>,
}

impl BlockActorCell {
    pub fn new(key: BlockActorKey, local: [i32; 3], skull: Option<SkullPose>) -> Self {
        Self { key, local, skull }
    }
}
