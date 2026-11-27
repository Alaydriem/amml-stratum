//! The ghost in the world render: the renderer that takes core's frames, builds the mesh slots with the loader's block
//! mesher and draws them, the palette's block handles, the mini ghost group it builds into, and the block entity ghosts
//! of the slots in view.

pub mod block_actor_cell;
pub mod block_actor_index;
pub mod block_actor_key;
pub mod block_actor_pool;
pub mod block_resolver;
pub mod ghost_renderer;
pub mod group_tracker;
pub mod skull_pose_map;

pub use block_actor_cell::BlockActorCell;
pub use block_actor_index::BlockActorIndex;
pub use block_actor_key::BlockActorKey;
pub use block_actor_pool::BlockActorPool;
pub use block_resolver::BlockResolver;
pub use ghost_renderer::GhostRenderer;
pub use group_tracker::GroupTracker;
pub use skull_pose_map::SkullPoseMap;
