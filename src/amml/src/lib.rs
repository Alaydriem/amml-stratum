//! @alaydriem/stratum as an AMML mod on the stable mod API: the lifecycle and listeners, the per-tick reader and world
//! key, the import picker, the core services, the ghost render through the loader's block mesher and world canvas,
//! easy place and the build through the loader's player, inventory and command calls, and, with the `gate` feature,
//! the in-game proof of those calls.

#![deny(unsafe_code)]

pub mod build;
pub mod easy_place;
pub mod ghost;
pub mod services;
pub mod stratum_mod;
pub mod traits;
pub mod types;

pub use stratum_mod::StratumMod;

amml_api::amml_mod!(StratumMod);
