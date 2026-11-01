use stratum_core::errors::GhostError;
use stratum_core::services::{GhostAssembler, NbtReader};
use stratum_core::structs::ghost::{GhostBuild, GhostPalette, GhostPiece};
use stratum_core::structs::layout::LayoutIssue;
use stratum_core::structs::library::{ProjectId, ProjectKind, StructureId};
use stratum_core::structs::marker::{Marker, MarkerTarget, Rotation};
use stratum_core::structs::nbt::{NbtLimits, NbtTag};
use stratum_core::structs::structure::{BlockLayer, BlockState, BlockStates, StateValue, Structure};

use crate::support::GhostFixture;

#[test]
fn structure_marker_places_cells_at_the_marker_origin() {
    let structure = GhostFixture::structure([2, 1, 1], &["stone", "oak_stairs"]);
    let marker = GhostFixture::marker([-5, 70, 3], MarkerTarget::Structure(StructureId(1)));
    let loaded = GhostFixture::loaded(&[(1, structure)]);
    let library = GhostFixture::library(ProjectKind::Normal, &[]);

    let build = GhostAssembler::assemble(&marker, &library, &loaded, 9).unwrap();

    let stone = build.block_at([-5, 70, 3]).unwrap();
    let stairs = build.block_at([-4, 70, 3]).unwrap();
    assert_ne!(stone, stairs);
    assert_eq!(build.palette.entries[usize::from(stone)].name, "minecraft:stone");
    assert_eq!(build.palette.entries[usize::from(stairs)].name, "minecraft:oak_stairs");
    assert_eq!(build.block_at([-3, 70, 3]), None);
    assert_eq!(build.block_at([-6, 70, 3]), None);
    assert_eq!(build.block_at([-5, 71, 3]), None);
    assert_eq!(build.world_min, [-5, 70, 3]);
    assert_eq!(build.world_max, [-4, 70, 3]);
    assert_eq!(build.revision, 9);
}

#[test]
fn big_build_pieces_sit_at_their_layout_offsets() {
    let library =
        GhostFixture::library(ProjectKind::BigBuild, &[(1, [100, 64, 0], [2, 1, 1]), (2, [102, 64, 0], [2, 1, 1])]);
    let loaded = GhostFixture::loaded(&[
        (1, GhostFixture::structure([2, 1, 1], &["stone", "dirt"])),
        (2, GhostFixture::structure([2, 1, 1], &["oak_planks", "glass"])),
    ]);
    let marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Project(ProjectId(1)));

    let build = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();

    let names: Vec<&str> = (0..4)
        .map(|x| {
            let id = build.block_at([x, 0, 0]).unwrap();
            build.palette.entries[usize::from(id)].name.as_str()
        })
        .collect();
    assert_eq!(names, ["minecraft:stone", "minecraft:dirt", "minecraft:oak_planks", "minecraft:glass"]);
    assert_eq!(build.block_at([4, 0, 0]), None);
    assert_eq!(build.world_max, [3, 0, 0]);
}

#[test]
fn equal_block_states_share_one_palette_id_across_pieces() {
    let library = GhostFixture::library(ProjectKind::BigBuild, &[(1, [0, 0, 0], [2, 1, 1]), (2, [2, 0, 0], [2, 1, 1])]);
    let loaded = GhostFixture::loaded(&[
        (1, GhostFixture::structure([2, 1, 1], &["stone", "dirt"])),
        (2, GhostFixture::structure([2, 1, 1], &["dirt", "stone"])),
    ]);
    let marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Project(ProjectId(1)));

    let build = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();

    assert_eq!(build.palette.entries.len(), 3);
    assert_eq!(build.block_at([0, 0, 0]), build.block_at([3, 0, 0]));
    assert_eq!(build.block_at([1, 0, 0]), build.block_at([2, 0, 0]));
}

#[test]
fn same_name_with_different_states_gets_different_ids() {
    let mut structure = (*GhostFixture::structure([2, 1, 1], &["oak_stairs", "oak_stairs"])).clone();
    structure.palette = vec![
        BlockState::new(
            "minecraft:oak_stairs".to_string(),
            BlockStates::new(vec![("facing".to_string(), StateValue::Int(0))]),
        ),
        BlockState::new(
            "minecraft:oak_stairs".to_string(),
            BlockStates::new(vec![("facing".to_string(), StateValue::Int(1))]),
        ),
    ];
    structure.primary = BlockLayer::new(vec![0, 1]);
    let loaded = GhostFixture::loaded(&[(1, structure.into())]);
    let marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Structure(StructureId(1)));
    let library = GhostFixture::library(ProjectKind::Normal, &[]);

    let build = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();

    assert_ne!(build.block_at([0, 0, 0]), build.block_at([1, 0, 0]));
    assert_eq!(build.palette.entries.len(), 3);
}

#[test]
fn air_and_structure_void_have_no_id_and_no_cell() {
    let structure = GhostFixture::structure([4, 1, 1], &["stone", "air", "structure_void", ""]);
    let loaded = GhostFixture::loaded(&[(1, structure)]);
    let marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Structure(StructureId(1)));
    let library = GhostFixture::library(ProjectKind::Normal, &[]);

    let build = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();

    assert_eq!(build.palette.entries.len(), 2);
    assert_eq!(build.block_at([0, 0, 0]), Some(0));
    assert_eq!(build.block_at([1, 0, 0]), None);
    assert_eq!(build.block_at([2, 0, 0]), None);
    assert_eq!(build.block_at([3, 0, 0]), None);
}

#[test]
fn combined_palette_past_the_limit_is_refused() {
    let library = GhostFixture::library(ProjectKind::Normal, &[]);
    let marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Structure(StructureId(1)));

    let at_limit = GhostFixture::loaded(&[(1, GhostFixture::distinct(GhostPalette::MAX_ENTRIES - 1))]);
    let build = GhostAssembler::assemble(&marker, &library, &at_limit, 1).unwrap();
    assert_eq!(build.palette.entries.len(), GhostPalette::MAX_ENTRIES);
    assert_eq!(build.block_at([0, 0, 65_532]), Some(65_532));

    let past = GhostFixture::loaded(&[(1, GhostFixture::distinct(usize::from(u16::MAX)))]);
    let refused = GhostAssembler::assemble(&marker, &library, &past, 1).unwrap_err();
    assert!(matches!(refused, GhostError::PaletteTooLarge { .. }));
}

#[test]
fn a_palette_filled_to_the_limit_leaves_no_room_for_the_marker_entry() {
    let library = GhostFixture::library(ProjectKind::Normal, &[]);
    let marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Structure(StructureId(1)));
    let full = GhostFixture::loaded(&[(1, GhostFixture::distinct(GhostPalette::MAX_ENTRIES))]);

    let refused = GhostAssembler::assemble(&marker, &library, &full, 1).unwrap_err();

    assert!(matches!(refused, GhostError::PaletteTooLarge { entries } if entries == GhostPalette::MAX_ENTRIES + 1));
}

#[test]
fn palette_entries_no_cell_uses_get_no_id() {
    let mut structure = (*GhostFixture::structure([1, 1, 1], &["stone"])).clone();
    structure.palette.push(BlockState::new("minecraft:dirt".to_string(), BlockStates::default()));
    let loaded = GhostFixture::loaded(&[(1, structure.into())]);
    let marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Structure(StructureId(1)));
    let library = GhostFixture::library(ProjectKind::Normal, &[]);

    let build = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();

    assert_eq!(build.palette.entries.len(), 2);
}

#[test]
fn missing_structure_is_reported_not_panicked() {
    let library = GhostFixture::library(ProjectKind::BigBuild, &[(1, [0, 0, 0], [2, 1, 1]), (2, [2, 0, 0], [2, 1, 1])]);
    let loaded = GhostFixture::loaded(&[(1, GhostFixture::structure([2, 1, 1], &["stone", "dirt"]))]);
    let marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Project(ProjectId(1)));

    let error = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap_err();

    assert_eq!(error, GhostError::NotLoaded(StructureId(2)));
}

#[test]
fn normal_project_shows_the_selected_structure() {
    let library = GhostFixture::library(
        ProjectKind::Normal,
        &[(10, [0, 0, 0], [1, 1, 1]), (11, [0, 0, 0], [1, 1, 1]), (12, [0, 0, 0], [1, 1, 1])],
    );
    let loaded = GhostFixture::loaded(&[
        (10, GhostFixture::structure([1, 1, 1], &["stone"])),
        (11, GhostFixture::structure([1, 1, 1], &["dirt"])),
        (12, GhostFixture::structure([1, 1, 1], &["glass"])),
    ]);
    let mut marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Project(ProjectId(1)));

    marker.view.selected = 1;
    assert_eq!(GhostAssembler::needed(&marker, &library).unwrap(), vec![StructureId(11)]);
    let build = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();
    assert_eq!(build.palette.entries[0].name, "minecraft:dirt");

    marker.view.selected = 99;
    assert_eq!(GhostAssembler::needed(&marker, &library).unwrap(), vec![StructureId(12)]);
}

#[test]
fn needed_lists_what_each_target_requires() {
    let big = GhostFixture::library(ProjectKind::BigBuild, &[(1, [0, 0, 0], [2, 1, 1]), (2, [2, 0, 0], [2, 1, 1])]);
    let project = GhostFixture::marker([0, 0, 0], MarkerTarget::Project(ProjectId(1)));
    assert_eq!(GhostAssembler::needed(&project, &big).unwrap(), vec![StructureId(1), StructureId(2)]);

    let single = GhostFixture::marker([0, 0, 0], MarkerTarget::Structure(StructureId(7)));
    assert_eq!(GhostAssembler::needed(&single, &big).unwrap(), vec![StructureId(7)]);

    let unknown = GhostFixture::marker([0, 0, 0], MarkerTarget::Project(ProjectId(5)));
    assert_eq!(GhostAssembler::needed(&unknown, &big), Err(GhostError::MissingTarget));

    let empty = GhostFixture::library(ProjectKind::Normal, &[]);
    assert_eq!(GhostAssembler::needed(&project, &empty), Err(GhostError::MissingTarget));
}

#[test]
fn unplaceable_big_build_reports_the_layout_issue() {
    let mut library = GhostFixture::library(ProjectKind::BigBuild, &[(1, [0, 0, 0], [2, 1, 1])]);
    library.structures[0].world_origin = None;
    let marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Project(ProjectId(1)));

    let error = GhostAssembler::needed(&marker, &library).unwrap_err();

    assert_eq!(error, GhostError::Layout(LayoutIssue::MissingOrigin(vec![StructureId(1)])));
}

#[test]
fn entry_nbt_writes_name_states_and_version() {
    let mut structure = (*GhostFixture::structure([1, 1, 1], &["oak_stairs"])).clone();
    structure.palette[0].states = BlockStates::new(vec![
        ("facing".to_string(), StateValue::Int(2)),
        ("upside_down_bit".to_string(), StateValue::Bool(true)),
        ("wood_type".to_string(), StateValue::Text("oak".to_string())),
    ]);
    let loaded = GhostFixture::loaded(&[(1, structure.into())]);
    let marker = GhostFixture::marker([0, 0, 0], MarkerTarget::Structure(StructureId(1)));
    let library = GhostFixture::library(ProjectKind::Normal, &[]);
    let build = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();

    let mut bytes = vec![0xAA];
    build.palette.entry_nbt(0, &mut bytes).unwrap();
    assert_eq!(bytes[0], 0xAA);
    let (_, root) = NbtReader::read(&bytes[1..], NbtLimits::STRUCTURE).unwrap();

    assert_eq!(root.get("name"), Some(&NbtTag::String("minecraft:oak_stairs".to_string())));
    assert_eq!(root.get("version"), Some(&NbtTag::Int(18_168_865)));
    let Some(NbtTag::Compound(states)) = root.get("states") else { panic!("states is a compound") };
    assert_eq!(states.get("facing"), Some(&NbtTag::Int(2)));
    assert_eq!(states.get("upside_down_bit"), Some(&NbtTag::Byte(1)));
    assert_eq!(states.get("wood_type"), Some(&NbtTag::String("oak".to_string())));

    let mut untouched = Vec::new();
    build.palette.entry_nbt(5, &mut untouched).unwrap();
    assert!(untouched.is_empty());
}

#[test]
fn first_piece_holding_a_cell_decides_even_when_its_cell_is_empty() {
    let empty = GhostFixture::structure([1, 1, 1], &["air"]);
    let stone = GhostFixture::structure([2, 1, 1], &["stone", "stone"]);
    let mut palette =
        GhostPalette { entries: vec![BlockState::new("minecraft:stone".to_string(), BlockStates::default())] };
    let marker_block = palette.marker_id().unwrap();
    let build = |first: &std::sync::Arc<Structure>, second: &std::sync::Arc<Structure>| GhostBuild {
        pieces: vec![
            GhostPiece { structure: first.clone(), world_min: [0, 0, 0], remap: vec![GhostPalette::NONE] },
            GhostPiece { structure: second.clone(), world_min: [0, 0, 0], remap: vec![0] },
        ],
        palette: palette.clone(),
        marker_block,
        world_min: [0, 0, 0],
        world_max: [1, 0, 0],
        revision: 1,
    };

    let covered = build(&empty, &stone);
    assert_eq!(covered.block_at([0, 0, 0]), None);
    assert_eq!(covered.block_at([1, 0, 0]), Some(0));
}

// The name of the block the build draws at a world cell.
fn name_at(build: &GhostBuild, world: [i32; 3]) -> Option<&str> {
    build.block_at(world).map(|id| build.palette.entries[usize::from(id)].name.as_str())
}

#[test]
fn rotated_cell_lands_at_the_turned_position() {
    // Cells in X, Y, Z order with Z fastest: (0,0,0) stone, (0,0,1) dirt, (1,0,0) glass, (1,0,1) sand,
    // (2,0,0) gravel, (2,0,1) oak_planks.
    let structure = GhostFixture::structure([3, 1, 2], &["stone", "dirt", "glass", "sand", "gravel", "oak_planks"]);
    let loaded = GhostFixture::loaded(&[(1, structure)]);
    let library = GhostFixture::library(ProjectKind::Normal, &[]);
    let at = |rotation: Rotation| {
        let marker = Marker { rotation, ..GhostFixture::marker([10, 64, 20], MarkerTarget::Structure(StructureId(1))) };
        GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap()
    };

    let turned = at(Rotation::R90);
    assert_eq!(name_at(&turned, [10, 64, 20]), Some("minecraft:stone"));
    assert_eq!(name_at(&turned, [9, 64, 22]), Some("minecraft:oak_planks"));
    assert_eq!(name_at(&turned, [10, 64, 22]), Some("minecraft:gravel"));
    assert_eq!(name_at(&turned, [9, 64, 20]), Some("minecraft:dirt"));
    assert_eq!(turned.world_min, [9, 64, 20]);
    assert_eq!(turned.world_max, [10, 64, 22]);

    let half = at(Rotation::R180);
    assert_eq!(name_at(&half, [10, 64, 20]), Some("minecraft:stone"));
    assert_eq!(name_at(&half, [8, 64, 19]), Some("minecraft:oak_planks"));
    assert_eq!(half.world_min, [8, 64, 19]);

    let marker = Marker {
        rotation: Rotation::R90,
        ..GhostFixture::marker([-10, -5, -20], MarkerTarget::Structure(StructureId(1)))
    };
    let below = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();
    assert_eq!(name_at(&below, [-10, -5, -20]), Some("minecraft:stone"));
    assert_eq!(name_at(&below, [-11, -5, -18]), Some("minecraft:oak_planks"));
    assert_eq!(below.world_min, [-11, -5, -20]);
    assert_eq!(below.world_max, [-10, -5, -18]);

    let back = at(Rotation::R270);
    assert_eq!(name_at(&back, [10, 64, 20]), Some("minecraft:stone"));
    assert_eq!(name_at(&back, [11, 64, 18]), Some("minecraft:oak_planks"));
    assert_eq!(back.world_min, [10, 64, 18]);
    assert_eq!(back.world_max, [11, 64, 20]);
}

#[test]
fn big_build_pieces_turn_around_the_build_corner() {
    let library =
        GhostFixture::library(ProjectKind::BigBuild, &[(1, [100, 64, 0], [2, 1, 1]), (2, [102, 64, 0], [2, 1, 1])]);
    let loaded = GhostFixture::loaded(&[
        (1, GhostFixture::structure([2, 1, 1], &["stone", "dirt"])),
        (2, GhostFixture::structure([2, 1, 1], &["oak_planks", "glass"])),
    ]);
    let marker =
        Marker { rotation: Rotation::R90, ..GhostFixture::marker([5, 0, 5], MarkerTarget::Project(ProjectId(1))) };

    let build = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();

    let names: Vec<Option<&str>> = (5..9).map(|z| name_at(&build, [5, 0, z])).collect();
    assert_eq!(
        names,
        [Some("minecraft:stone"), Some("minecraft:dirt"), Some("minecraft:oak_planks"), Some("minecraft:glass")]
    );
    assert_eq!(build.world_min, [5, 0, 5]);
    assert_eq!(build.world_max, [5, 0, 8]);
    assert_eq!(build.pieces[1].world_min, [5, 0, 7]);
}

#[test]
fn assembled_build_has_a_marker_block() {
    let structure = GhostFixture::structure([2, 1, 1], &["stone", "oak_stairs"]);
    let marker = GhostFixture::marker([0, 64, 0], MarkerTarget::Structure(StructureId(1)));
    let loaded = GhostFixture::loaded(&[(1, structure)]);
    let library = GhostFixture::library(ProjectKind::Normal, &[]);

    let build = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();

    assert_eq!(build.palette.entries[usize::from(build.marker_block)], GhostPalette::marker_entry());
}

#[test]
fn reused_palette_keeps_one_marker_entry() {
    let structure = GhostFixture::structure([2, 1, 1], &["stone", "oak_stairs"]);
    let marker = GhostFixture::marker([0, 64, 0], MarkerTarget::Structure(StructureId(1)));
    let loaded = GhostFixture::loaded(&[(1, structure)]);
    let library = GhostFixture::library(ProjectKind::Normal, &[]);
    let first = GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap();
    let placed = GhostAssembler::placed(&marker, &library, &loaded).unwrap();
    let turned: Vec<_> = placed.iter().map(|piece| piece.structure.clone()).collect();

    let second = GhostAssembler::build(&placed, &turned, marker.origin, Rotation::R0, 2, Some(&first)).unwrap();

    assert_eq!(second.marker_block, first.marker_block);
    let markers = second.palette.entries.iter().filter(|entry| **entry == GhostPalette::marker_entry()).count();
    assert_eq!(markers, 1);
}
