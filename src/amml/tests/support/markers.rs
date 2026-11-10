use amm_core::structs::descriptor::Capability;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::marker::MarkerTarget;

use super::CoreSession;

// A marker that targets a structure the library does not hold: active and shown, with nothing to build.
pub struct Markers;

impl Markers {
    // The first in-world tick loads the world's markers; the place lands on the second.
    pub fn place(session: &CoreSession) {
        session.tick(&CoreSession::in_world());
        let place =
            LibraryAction::PlaceMarker { name: "Plot".to_string(), target: MarkerTarget::Structure(StructureId(7)) };
        session.shared().send(&mut vec![place]);
        session.tick(&CoreSession::in_world());
    }

    pub fn world_draw(calls: Vec<Capability>) -> usize {
        calls.iter().filter(|capability| **capability == Capability::WorldDraw).count()
    }
}
