use alaydriem_stratum::StratumMod;
use alaydriem_stratum::errors::GameLayoutError;
use alaydriem_stratum::types::{GhostCalls, StratumEntry};

use crate::support::{FakeGhost, FakeResolver};

fn calls(values: &[(amml_game::EntryId, u64)]) -> Result<GhostCalls, GameLayoutError> {
    let entries = StratumMod::resolve(&FakeResolver::new(values)).unwrap();
    GhostCalls::new(&entries, FakeGhost::heap_new, FakeGhost::heap_delete)
}

#[test]
fn the_registry_object_sits_0x18_into_the_owner_global() {
    let instance = FakeResolver::values()
        .into_iter()
        .find(|(id, _)| *id == StratumEntry::BLOCK_TYPE_REGISTRY_INSTANCE)
        .map(|(_, value)| value)
        .unwrap();

    let calls = calls(&FakeResolver::values()).unwrap();

    assert_eq!(calls.block_type_registry as u64, instance + 0x18);
}

#[test]
fn a_tessellator_anchor_off_the_mirror_is_refused() {
    let result = calls(&FakeResolver::with(StratumEntry::TESSELLATOR_TESSELLATING, 0x2A8));

    assert_eq!(
        result.unwrap_err(),
        GameLayoutError::Misfit { entry: "Tessellator.mTessellating", expected: 0x2A0, found: 0x2A8 }
    );
}

#[test]
fn a_block_cache_anchor_or_mesh_size_off_the_mirror_is_refused() {
    let cache = calls(&FakeResolver::with(StratumEntry::BLOCK_TESSELLATOR_BLOCK_CACHE, 0x6A8));
    let mesh = calls(&FakeResolver::with(StratumEntry::MESH_SIZE, 640));

    assert!(matches!(cache, Err(GameLayoutError::Misfit { entry: "BlockTessellator.mBlockCache", .. })));
    assert!(matches!(mesh, Err(GameLayoutError::Misfit { entry: "Mesh.size", .. })));
}

#[test]
fn a_2660_mesh_size_selects_the_second_mesh_mirror() {
    let result = calls(&FakeResolver::with(StratumEntry::MESH_SIZE, 608));

    assert!(result.is_ok());
}

#[test]
fn a_mesh_size_that_no_mirror_has_refuses_the_ghost_with_a_line_that_names_it() {
    let error = calls(&FakeResolver::with(StratumEntry::MESH_SIZE, 0x280)).unwrap_err();

    assert_eq!(error.to_string(), "Mesh.size is 0x280, the mirror needs 0x278");
}
