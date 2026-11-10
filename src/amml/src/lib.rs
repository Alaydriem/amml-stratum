//! @alaydriem/stratum as an AMML mod: the lifecycle, the per-tick reader and world key, the import picker, the core
//! services moved from the C ABI crate, the game layout mirrors and the wrappers of the ghost render path, and, with
//! the `gate` feature, the in-game check that builds and draws one mesh and looks up one block by NBT.

#![deny(unsafe_code)]

pub mod detours;
pub mod easy_place;
pub mod errors;
pub mod game;
pub mod ghost;
pub mod services;
pub mod stratum_mod;
pub mod traits;
pub mod types;

pub use stratum_mod::StratumMod;

amml_sdk::amml_mod!(StratumMod);
