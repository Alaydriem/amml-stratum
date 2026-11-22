use std::sync::Mutex;

use amml_api::{BlockHandle, Hud, LocalPlayer};
use stratum_core::structs::easy_place::UseCommand;

use crate::easy_place::{UseDriver, UseExecutor, UseFlags};
use crate::traits::UseCoreHolder;
use crate::types::UseOutcome;

/// The lock rule of a use call: the command is decided under the core and driver locks, carried out with no lock held
/// (`build_at` raises `BuildBlock` and the block change on the same stack, and their listeners take these locks), and
/// what it gave goes back under the locks again. The runtime runs the use press and the tick's use call through it.
pub struct UseCycle;

impl UseCycle {
    /// The use press: true cancels the game's handling of it. With the core or the driver held elsewhere the press goes
    /// to the game. `pick` gives the block of a Pick's palette id.
    #[allow(clippy::too_many_arguments)]
    pub fn press<H: UseCoreHolder>(
        core: &Mutex<H>,
        driver: &Mutex<UseDriver>,
        flags: &UseFlags,
        player: Option<&LocalPlayer<'_>>,
        hud: &Hud<'_>,
        key_held: bool,
        now: f64,
        pick: impl Fn(&UseCommand) -> Option<BlockHandle>,
    ) -> bool {
        let (command, counts) = {
            let Ok(mut core) = core.try_lock() else { return false };
            let Ok(mut driver) = driver.try_lock() else { return false };
            (driver.press(flags, core.use_core(), player, key_held, now), driver.counts())
        };
        match command {
            None | Some(UseCommand::Vanilla) => false,
            Some(UseCommand::Block) => true,
            Some(command) => {
                if let Some(player) = player {
                    Self::carry_out(core, driver, player, hud, &command, pick(&command), &counts);
                }
                true
            }
        }
    }

    /// Carries out `command` with no lock held, then hands the outcome to the driver and the core under their locks.
    /// The caller holds neither lock.
    pub fn carry_out<H: UseCoreHolder>(
        core: &Mutex<H>,
        driver: &Mutex<UseDriver>,
        player: &LocalPlayer<'_>,
        hud: &Hud<'_>,
        command: &UseCommand,
        pick: Option<BlockHandle>,
        counts: &[u8; 36],
    ) -> UseOutcome {
        let outcome = UseExecutor::run(player, hud, command, pick, counts);
        if (outcome.request.is_some() || outcome.not_sent.is_some())
            && let (Ok(mut core), Ok(mut driver)) = (core.lock(), driver.lock())
        {
            driver.apply(&outcome, core.use_core());
        }
        outcome
    }
}
