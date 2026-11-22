//! The build in the mod crate: it reads the conditions, answers the core's palette and kinds requests from the game
//! and sends the commands the core gives through the loader's command call. It decides nothing.

pub mod api_build_world;
pub mod build_executor;
pub mod conditions_reader;
pub mod kinds_scanner;
pub mod palette_probe;

pub use api_build_world::ApiBuildWorld;
pub use build_executor::BuildExecutor;
pub use conditions_reader::ConditionsReader;
pub use kinds_scanner::KindsScanner;
pub use palette_probe::PaletteProbe;
