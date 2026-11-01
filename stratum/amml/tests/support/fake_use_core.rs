use alaydriem_stratum::traits::UseCore;
use stratum_core::structs::easy_place::{InventorySlot, SpotWorld, UseCommand, UseProbe};

use crate::support::fake_use::LOG;

/// A core that aims at `spot` and answers every decide with `command`, logging each question in the order of the game
/// calls, and keeping the last probe, world and inventory it was given.
pub struct FakeUseCore {
    pub spot: Option<[i32; 3]>,
    pub command: UseCommand,
    pub probe: Option<UseProbe>,
    pub world: Option<SpotWorld>,
    pub slots: Vec<(Vec<u8>, u32)>,
    pub selected: u8,
}

impl FakeUseCore {
    pub fn new(command: UseCommand) -> Self {
        Self { spot: Some([0, 64, -2]), command, probe: None, world: None, slots: Vec::new(), selected: 0 }
    }
}

impl UseCore for FakeUseCore {
    fn aim(&mut self, probe: &UseProbe) -> Option<Option<[i32; 3]>> {
        LOG.lock().unwrap().push("aim".to_string());
        self.probe = Some(*probe);
        Some(self.spot)
    }

    fn decide(
        &mut self,
        _probe: &UseProbe,
        world: Option<&SpotWorld>,
        inventory: &[InventorySlot<'_>],
        selected: u8,
    ) -> Option<UseCommand> {
        LOG.lock().unwrap().push("decide".to_string());
        self.world = world.copied();
        self.slots = inventory.iter().map(|slot| (slot.item.to_vec(), slot.count)).collect();
        self.selected = selected;
        Some(self.command)
    }

    fn released(&mut self) {
        LOG.lock().unwrap().push("released".to_string());
    }

    fn not_sent(&mut self, spot: [i32; 3]) {
        LOG.lock().unwrap().push(format!("not_sent {} {} {}", spot[0], spot[1], spot[2]));
    }
}
