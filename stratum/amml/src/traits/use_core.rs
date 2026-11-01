use stratum_core::structs::easy_place::{InventorySlot, SpotWorld, UseCommand, UseProbe};

// What a use call asks the core, in the order the call asks it. Each answer is None when the core cannot answer (a
// panic poisoned it); the running core answers through its lock, a test gives fixed answers
pub trait UseCore {
    // the ghost spot under the crosshair; Some(None) when there is none
    fn aim(&mut self, probe: &UseProbe) -> Option<Option<[i32; 3]>>;

    // what to do with this use
    fn decide(
        &mut self,
        probe: &UseProbe,
        world: Option<&SpotWorld>,
        inventory: &[InventorySlot<'_>],
        selected: u8,
    ) -> Option<UseCommand>;

    // the use key went up
    fn released(&mut self);

    // the Place or Select and Place command for `spot` was not carried out
    fn not_sent(&mut self, spot: [i32; 3]);
}
