//! The ghost on the render thread: the renderer that takes core's frames, builds the mesh slots and draws them, the
//! tessellator and group buffer it builds with, the palette's block lookups, and the block listener on the client
//! region that tells core which blocks changed.

pub mod block_change_listener;
pub mod block_resolver;
pub mod ghost_group_buffer;
pub mod ghost_renderer;
pub mod ghost_tessellator;

pub use block_change_listener::BlockChangeListener;
pub use block_resolver::BlockResolver;
pub use ghost_group_buffer::GhostGroupBuffer;
pub use ghost_renderer::GhostRenderer;
pub use ghost_tessellator::GhostTessellator;
