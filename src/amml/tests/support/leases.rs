use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::ModEvent;

use super::CoreSession;

// The lease calls the fake manager recorded.
pub struct Leases;

impl Leases {
    pub fn acquired() -> Vec<Capability> {
        FakeTable::with(|state| state.acquired.clone())
    }

    pub fn released() -> Vec<Capability> {
        FakeTable::with(|state| state.released.clone())
    }

    // The session opened, then got `event` on an in-world tick.
    pub fn closed_by(event: ModEvent) -> CoreSession {
        let session = CoreSession::new();
        session.open();
        FakeTable::push_event(event);
        session.tick(&CoreSession::in_world());
        session
    }
}
