//! The build in the mod crate: it reads the conditions, answers the core's palette and kinds requests from the game
//! and sends the commands the core gives, as the game's command request packet. It decides nothing.

pub mod build_executor;
pub mod command_packet;
pub mod conditions_reader;
pub mod game_build_world;
pub mod kinds_scanner;
pub mod palette_probe;

pub use build_executor::BuildExecutor;
pub use command_packet::CommandPacket;
pub use conditions_reader::ConditionsReader;
pub use game_build_world::GameBuildWorld;
pub use kinds_scanner::KindsScanner;
pub use palette_probe::PaletteProbe;
