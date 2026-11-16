use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use alaydriem_stratum::types::BlockActorCalls;
use amml_game::BlockPos;
use stratum_core::structs::ghost::{
    ChestPair, GhostBlockEntity, GhostBlockEntityTable, GhostDraw, GhostFrame, GhostJob, GhostJobMode, SkullPose,
};

use crate::support::fake_block_actor::{
    CREATE_CALLS, DESTROY_CALLS, FULL_NAME_OFFSET, IS_TYPE_ASKED, IS_TYPE_CALLS, LOAD_CALLS, NOT_A_CHEST,
    PAIRED_OFFSET, READ_BYTES, Rendered, SKULL_SLOT,
};
use crate::support::fake_ghost::{TESSELLATE_EMPTY, WORLD_BLOCK_AT};
use crate::support::{FakeBlockActor, FakeBuffer, GhostRig};

const STONE: &str = "minecraft:stone";
const DISPATCHER: u64 = 0xD15;
const LEVEL: *mut c_void = 0x1E7E1 as *mut c_void;
const CAMERA_TARGET: [f32; 3] = [0.5, 60.0, -2.25];
const ORIGIN: [i32; 3] = [-3, 64, 5];

// a render context laid out as the fake calls name it: the dispatcher at 0x30, the Impl pointer at 0xa8, and the
// camera target at 0x20 of the Impl
struct FakeContext {
    ctx: FakeBuffer,
    _context_impl: FakeBuffer,
}

impl FakeContext {
    fn new() -> Self {
        let mut context_impl = FakeBuffer::new(0x40);
        for (axis, value) in CAMERA_TARGET.iter().enumerate() {
            context_impl.put_f32(0x20 + axis * 4, *value);
        }
        let mut ctx = FakeBuffer::new(0x100);
        ctx.put_u64(0x30, DISPATCHER);
        ctx.put_u64(0xa8, context_impl.as_mut_ptr() as u64);
        Self { ctx, _context_impl: context_impl }
    }
}

// a Replace job of two cells of palette id 0 along X at `origin`, both drawn with the block actor renderer: the first
// with no NBT, the second with entry 0 of the table
fn pair_job(slot: u32, serial: u32, origin: [i32; 3]) -> GhostJob {
    let mut job = GhostRig::job(slot, serial, 0, origin);
    job.dims = [2, 1, 1];
    job.slot_size = [2, 1, 1];
    // padded grid 4 x 3 x 3, X fastest, then Z, then Y
    job.cells = vec![u16::MAX; 36];
    job.cells[17] = 0;
    job.cells[18] = 0;
    job.block_entities =
        vec![GhostBlockEntity::new(17, GhostBlockEntity::NONE, None, None), GhostBlockEntity::new(18, 0, None, None)];
    job
}

// a job like `pair_job` whose two cells share entry 0 of the table: the first the lead of a large chest that pairs
// along +Z, the second a single chest
fn lead_and_single_job(slot: u32, serial: u32, origin: [i32; 3]) -> GhostJob {
    let mut job = pair_job(slot, serial, origin);
    job.block_entities =
        vec![GhostBlockEntity::new(17, 0, Some(ChestPair::PlusZ), None), GhostBlockEntity::new(18, 0, None, None)];
    job
}

// a one-cell job at `origin` whose cell is drawn with the block actor renderer
fn single_job(slot: u32, serial: u32, origin: [i32; 3]) -> GhostJob {
    let mut job = GhostRig::job(slot, serial, 0, origin);
    job.block_entities = vec![GhostBlockEntity::new(13, GhostBlockEntity::NONE, None, None)];
    job
}

// a rig in world A that ran one frame of `jobs` and `draws`, with a table of one NBT entry
fn rig_with(jobs: Vec<GhostJob>, draws: Vec<GhostDraw>) -> GhostRig {
    let mut rig = GhostRig::new();
    let mut frame = GhostRig::ghost_frame(1, 1, &[STONE]);
    frame.jobs = jobs;
    frame.draws = draws;
    frame.block_entities = Some(Arc::new(GhostBlockEntityTable::new(vec![vec![10, 0, 0]])));
    rig.publish(&frame);
    rig.frame(Some(GhostRig::REGION_A));
    rig
}

// the block-entity pass with the client's region still REGION_A
fn draw(rig: &mut GhostRig, context: &mut FakeContext) -> u32 {
    draw_in(rig, context, GhostRig::REGION_A)
}

// the block-entity pass with the client's current region `region`
fn draw_in(rig: &mut GhostRig, context: &mut FakeContext, region: usize) -> u32 {
    draw_with(rig, context, region, LEVEL)
}

// the block-entity pass with the client's current region `region` and level `level`
fn draw_with(rig: &mut GhostRig, context: &mut FakeContext, region: usize, level: *mut c_void) -> u32 {
    draw_calls(rig, context, region, level, &FakeBlockActor::calls())
}

// the block-entity pass with the client's current region `region`, level `level` and the calls `calls`
#[allow(unsafe_code)]
fn draw_calls(
    rig: &mut GhostRig,
    context: &mut FakeContext,
    region: usize,
    level: *mut c_void,
    calls: &BlockActorCalls,
) -> u32 {
    let ctx = context.ctx.as_mut_ptr();
    // SAFETY: fake game code for every call and a fake render context laid out as the calls name it
    unsafe { rig.renderer.draw_block_entities(calls, &rig.ghost, &SKULL_SLOT, ctx, region as *mut c_void, level) }
}

// a one-cell job at `origin` whose cell is drawn with the block actor renderer from entry 0 of the table
fn entry_job(slot: u32, serial: u32, origin: [i32; 3]) -> GhostJob {
    let mut job = GhostRig::job(slot, serial, 0, origin);
    job.block_entities = vec![GhostBlockEntity::new(13, 0, None, None)];
    job
}

// a frame of generation 1 in world 1 that draws slot 0 at ORIGIN, with the palette again and `table`
fn table_frame(table: Vec<u8>) -> GhostFrame {
    let mut frame = GhostRig::ghost_frame(1, 1, &[STONE]);
    frame.draws = vec![GhostRig::draw(0, ORIGIN)];
    frame.block_entities = Some(Arc::new(GhostBlockEntityTable::new(vec![table])));
    frame
}

fn read_bytes() -> Vec<u8> {
    READ_BYTES.lock().unwrap().clone()
}

fn world_positions(renders: &[Rendered]) -> Vec<BlockPos> {
    renders.iter().map(|render| render.world_pos).collect()
}

#[test]
fn each_visible_cell_draws_once_with_a_camera_relative_position() {
    let mut rig = rig_with(vec![pair_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();

    let drawn = draw(&mut rig, &mut context);

    assert_eq!(drawn, 2);
    let renders = FakeBlockActor::renders();
    let block = rig.renderer.palette_block(0) as usize;
    let first = BlockPos { x: -3, y: 64, z: 5 };
    let second = BlockPos { x: -2, y: 64, z: 5 };
    assert_eq!(world_positions(&renders), [first, second]);
    assert_eq!(renders[0].render_pos, [-3.5, 4.0, 7.25]);
    assert_eq!(renders[1].render_pos, [-2.5, 4.0, 7.25]);
    for render in &renders {
        assert_eq!(render.dispatcher, DISPATCHER as usize);
        assert_eq!(render.ctx, context.ctx.as_mut_ptr() as usize);
        assert_eq!(render.region, GhostRig::REGION_A);
        assert_eq!(render.block, block);
        assert_ne!(render.component, 0);
        assert_eq!(render.alpha, 0, "the opaque layer");
        assert_eq!(render.material, [0, 0], "no forced material");
        assert_eq!(render.texture, 0, "no forced texture");
        assert_eq!(render.breaking, 0);
        assert_eq!(render.metadata_engaged, 0, "no render metadata");
    }
    assert_eq!(FakeBlockActor::count(&CREATE_CALLS), 1, "the cell with no NBT gets a default actor");
    assert_eq!(FakeBlockActor::count(&LOAD_CALLS), 1, "the cell with NBT is loaded from it");
    assert_eq!(FakeBlockActor::seen().map(|seen| seen.level), Some(LEVEL as usize));

    let drawn_again = draw(&mut rig, &mut context);

    assert_eq!(drawn_again, 2);
    assert_eq!(
        (FakeBlockActor::count(&CREATE_CALLS), FakeBlockActor::count(&LOAD_CALLS)),
        (1, 1),
        "the next frame reuses the actors"
    );
    assert_eq!(rig.renderer.block_actors(), 2);
}

#[test]
fn a_skipped_cell_is_not_drawn() {
    let mut rig = GhostRig::new();
    rig.publish(&GhostRig::ghost_frame(1, 1, &[STONE]));
    rig.frame(Some(GhostRig::REGION_A));
    // the world already holds the ghost's block at the job's first cell
    let block = rig.renderer.palette_block(0) as usize;
    *WORLD_BLOCK_AT.lock().unwrap() = Some((BlockPos { x: ORIGIN[0], y: ORIGIN[1], z: ORIGIN[2] }, block));
    let mut frame = GhostRig::ghost_frame(1, 1, &[STONE]);
    frame.jobs = vec![pair_job(0, 1, ORIGIN)];
    frame.draws = vec![GhostRig::draw(0, ORIGIN)];
    rig.publish(&frame);
    rig.frame(Some(GhostRig::REGION_A));
    let mut context = FakeContext::new();

    let drawn = draw(&mut rig, &mut context);

    assert_eq!(drawn, 1);
    assert_eq!(world_positions(&FakeBlockActor::renders()), [BlockPos { x: -2, y: 64, z: 5 }]);
}

#[test]
fn a_slot_outside_the_visible_order_is_not_drawn() {
    let far = [40, 64, -7];
    let mut rig = rig_with(vec![single_job(0, 1, ORIGIN), single_job(1, 2, far)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();

    let drawn = draw(&mut rig, &mut context);

    assert_eq!(drawn, 1);
    assert_eq!(world_positions(&FakeBlockActor::renders()), [BlockPos { x: -3, y: 64, z: 5 }]);
}

#[test]
fn leave_clears_the_pool() {
    let mut rig = rig_with(vec![single_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();
    draw(&mut rig, &mut context);
    assert_eq!(rig.renderer.block_actors(), 1);

    rig.frame(None);

    assert_eq!(rig.renderer.block_actors(), 0);
    assert_eq!(DESTROY_CALLS.load(Ordering::SeqCst), 1, "the actor's use was given back");
    assert_eq!(draw(&mut rig, &mut context), 0, "nothing draws outside a world");
}

#[test]
fn a_pass_after_the_region_changed_draws_nothing_until_the_frame_joins_it() {
    let mut rig = rig_with(vec![single_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();

    let drawn_elsewhere = draw_in(&mut rig, &mut context, GhostRig::REGION_B);

    assert_eq!(drawn_elsewhere, 0);
    assert!(FakeBlockActor::renders().is_empty(), "the old region never reaches the dispatcher");
    assert_eq!(draw_in(&mut rig, &mut context, GhostRig::REGION_A), 1);
    assert_eq!(FakeBlockActor::renders()[0].region, GhostRig::REGION_A);
}

#[test]
fn a_slot_with_no_mesh_still_draws_its_block_entities() {
    TESSELLATE_EMPTY.store(true, Ordering::SeqCst);
    let mut rig = rig_with(vec![single_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();

    let drawn = draw(&mut rig, &mut context);

    assert_eq!(drawn, 1, "the cull kept the slot for its block entity");
    let reports = rig.reports();
    assert!(!reports.is_empty());
    assert!(reports.iter().all(|report| report.meshes_drawn == 0), "no mesh was drawn for the slot");
}

#[test]
fn a_miniature_draw_draws_no_block_entity_models() {
    let mini = GhostDraw { slot: 0, origin: ORIGIN, offset: [0; 3], scale: 0.5 };
    let mut rig = rig_with(vec![single_job(0, 1, ORIGIN)], vec![mini]);
    let mut context = FakeContext::new();

    assert_eq!(draw(&mut rig, &mut context), 0);
    assert!(FakeBlockActor::renders().is_empty());
}

#[test]
fn a_group_mesh_in_a_slot_drops_the_slot_block_entity_cells() {
    let mut rig = rig_with(vec![single_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    // the group's parts come one frame each, as the scheduler sends them
    let parts =
        [GhostRig::group_job(0, 2, GhostJobMode::Append, 7, 0), GhostRig::group_job(0, 3, GhostJobMode::Finish, 7, 1)];
    for part in parts {
        let mut frame = GhostRig::ghost_frame(1, 1, &[STONE]);
        frame.palette = None;
        frame.jobs = vec![part];
        frame.draws = vec![GhostRig::draw(0, ORIGIN)];
        rig.publish(&frame);
        rig.frame(Some(GhostRig::REGION_A));
    }
    let built: Vec<_> = rig.reports().into_iter().flat_map(|report| report.built).collect();
    assert!(built.iter().any(|entry| entry.serial == 3 && entry.ok), "the group mesh was uploaded");
    let mut context = FakeContext::new();

    assert_eq!(draw(&mut rig, &mut context), 0);
    assert!(FakeBlockActor::renders().is_empty());
}

#[test]
fn a_new_table_drops_every_old_key_and_the_rebuilt_cell_loads_the_new_entry() {
    let mut rig = rig_with(vec![entry_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();
    assert_eq!(draw(&mut rig, &mut context), 1);
    assert_eq!(read_bytes(), [10, 0, 0]);

    // a turn: the same generation sends the palette again with a table whose entry 0 is other NBT, and the slot is
    // not rebuilt yet
    rig.publish(&table_frame(vec![20, 0, 0]));
    rig.frame(Some(GhostRig::REGION_A));

    assert_eq!(draw(&mut rig, &mut context), 0, "no cell keeps a key of the old table");
    assert_eq!(FakeBlockActor::count(&LOAD_CALLS), 1);
    assert_eq!(FakeBlockActor::renders().len(), 1);

    let mut rebuilt = GhostRig::ghost_frame(1, 1, &[STONE]);
    rebuilt.palette = None;
    rebuilt.jobs = vec![entry_job(0, 2, ORIGIN)];
    rebuilt.draws = vec![GhostRig::draw(0, ORIGIN)];
    rig.publish(&rebuilt);
    rig.frame(Some(GhostRig::REGION_A));

    assert_eq!(draw(&mut rig, &mut context), 1);
    assert_eq!(FakeBlockActor::count(&LOAD_CALLS), 2, "a new actor for the new entry");
    assert_eq!(read_bytes(), [20, 0, 0]);
}

#[test]
fn a_job_latched_before_a_new_table_draws_no_block_entity_models() {
    let mut rig = GhostRig::new();
    let mut first = table_frame(vec![10, 0, 0]);
    first.jobs = vec![entry_job(0, 1, ORIGIN)];
    rig.publish(&first);
    // the turn comes before the render thread took the first frame
    rig.publish(&table_frame(vec![20, 0, 0]));
    rig.frame(Some(GhostRig::REGION_A));
    let mut context = FakeContext::new();

    assert_eq!(draw(&mut rig, &mut context), 0);
    assert_eq!(FakeBlockActor::count(&LOAD_CALLS), 0);
}

#[test]
fn a_pass_under_a_moved_world_epoch_draws_nothing_even_in_the_same_region() {
    let mut rig = rig_with(vec![single_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();
    // core is in the next world, whose region sits at the old address; no render frame ran since
    rig.publish(&GhostRig::ghost_frame(2, 2, &[STONE]));

    assert_eq!(draw(&mut rig, &mut context), 0);
    assert!(FakeBlockActor::renders().is_empty(), "the old world's blocks and actors never reach the dispatcher");
}

#[test]
fn a_load_with_no_level_is_not_remembered_as_failed() {
    let mut rig = rig_with(vec![entry_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();

    assert_eq!(draw_with(&mut rig, &mut context, GhostRig::REGION_A, std::ptr::null_mut()), 0);
    assert_eq!(draw(&mut rig, &mut context), 1, "the cell loads once the level is there");
    assert_eq!(FakeBlockActor::count(&LOAD_CALLS), 1);
}

#[test]
fn a_single_chest_is_not_marked_large() {
    let mut rig = rig_with(vec![entry_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();

    assert_eq!(draw(&mut rig, &mut context), 1);

    let actor = FakeBlockActor::actor_of(&FakeBlockActor::renders()[0]);
    assert_eq!(FakeBlockActor::large_chest(actor), (0, BlockPos { x: 0, y: 0, z: 0 }, 0));
}

#[test]
#[allow(unsafe_code)]
fn a_lead_and_a_single_chest_do_not_share_an_actor() {
    let mut rig = rig_with(vec![lead_and_single_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();

    assert_eq!(draw(&mut rig, &mut context), 2);

    let renders = FakeBlockActor::renders();
    let (lead, single) = (FakeBlockActor::actor_of(&renders[0]), FakeBlockActor::actor_of(&renders[1]));
    assert_ne!(lead, single);
    assert_eq!(FakeBlockActor::count(&LOAD_CALLS), 2, "one actor for each key");
    let lead_pos = BlockPos { x: ORIGIN[0], y: ORIGIN[1], z: ORIGIN[2] };
    assert_eq!(
        FakeBlockActor::large_chest(lead),
        (lead, BlockPos { z: lead_pos.z + 1, ..lead_pos }, 0x02),
        "the lead points at itself, its partner one block along +Z"
    );
    assert_eq!(FakeBlockActor::large_chest(single), (0, BlockPos { x: 0, y: 0, z: 0 }, 0));
    assert_eq!(IS_TYPE_ASKED.load(Ordering::SeqCst), 2, "the lead's actor was asked whether it is a chest");
    assert_eq!(FakeBlockActor::count(&IS_TYPE_CALLS), 1, "only the lead's actor is asked");

    // the pair is written when the actor is made, not on each frame
    // SAFETY: the fake actor the drawer holds, larger than the field
    unsafe { (lead as *mut u8).add(PAIRED_OFFSET).cast::<usize>().write_unaligned(0) };
    assert_eq!(draw(&mut rig, &mut context), 2);
    assert_eq!(FakeBlockActor::large_chest(lead).0, 0);
    assert_eq!(rig.renderer.block_actors(), 2);
}

#[test]
fn a_lead_key_whose_actor_is_not_a_chest_gets_no_writes() {
    NOT_A_CHEST.store(true, Ordering::SeqCst);
    let mut rig = rig_with(vec![lead_and_single_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();

    assert_eq!(draw(&mut rig, &mut context), 2, "both cells still draw, the lead as a single actor");

    let lead = FakeBlockActor::actor_of(&FakeBlockActor::renders()[0]);
    assert_eq!(FakeBlockActor::count(&IS_TYPE_CALLS), 1);
    assert_eq!(FakeBlockActor::large_chest(lead), (0, BlockPos { x: 0, y: 0, z: 0 }, 0));
}

#[test]
fn a_lead_key_with_the_large_chests_off_draws_as_a_single_chest_with_no_writes() {
    let mut rig = rig_with(vec![lead_and_single_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();
    let calls = BlockActorCalls { chest: None, ..FakeBlockActor::calls() };

    assert_eq!(draw_calls(&mut rig, &mut context, GhostRig::REGION_A, LEVEL, &calls), 2, "both cells draw");

    let lead = FakeBlockActor::actor_of(&FakeBlockActor::renders()[0]);
    assert_eq!(FakeBlockActor::count(&IS_TYPE_CALLS), 0, "no chest check without the chest fields");
    assert_eq!(FakeBlockActor::large_chest(lead), (0, BlockPos { x: 0, y: 0, z: 0 }, 0));
}

// a job like `pair_job` whose first cell is a skull on the floor turned 45 degrees and whose second cell is no skull
fn skull_and_plain_job(slot: u32, serial: u32, origin: [i32; 3]) -> GhostJob {
    let mut job = pair_job(slot, serial, origin);
    job.block_entities = vec![
        GhostBlockEntity::new(17, GhostBlockEntity::NONE, None, Some(SkullPose::new(1, 45.0))),
        GhostBlockEntity::new(18, GhostBlockEntity::NONE, None, None),
    ];
    job
}

#[test]
fn the_slot_holds_the_skull_pose_only_during_its_cell_draw() {
    let mut rig = rig_with(vec![skull_and_plain_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();

    assert_eq!(draw(&mut rig, &mut context), 2);

    let renders = FakeBlockActor::renders();
    let block_type = FakeBlockActor::seen().map(|seen| seen.block_type).unwrap();
    let name = block_type + FULL_NAME_OFFSET as usize;
    assert_eq!(renders[0].skull, Some((SkullPose::new(1, 45.0), name)), "the cell's pose and its block type name");
    assert_eq!(renders[1].skull, None, "a cell with no skull pose sees an empty slot");
}

#[test]
fn a_skull_cell_with_the_skulls_off_is_not_drawn() {
    let mut rig = rig_with(vec![skull_and_plain_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();
    let calls = BlockActorCalls { skull: None, ..FakeBlockActor::calls() };

    assert_eq!(draw_calls(&mut rig, &mut context, GhostRig::REGION_A, LEVEL, &calls), 1, "only the plain cell");

    let renders = FakeBlockActor::renders();
    assert_eq!(world_positions(&renders), vec![BlockPos { x: ORIGIN[0] + 1, y: ORIGIN[1], z: ORIGIN[2] }]);
    assert_eq!(renders[0].skull, None);
    assert_eq!(FakeBlockActor::count(&CREATE_CALLS), 1, "no actor is made for the skull cell");
}

#[test]
fn the_slot_is_empty_after_the_cell_draw() {
    let mut rig = rig_with(vec![skull_and_plain_job(0, 1, ORIGIN)], vec![GhostRig::draw(0, ORIGIN)]);
    let mut context = FakeContext::new();

    draw(&mut rig, &mut context);

    assert_eq!(SKULL_SLOT.get(), None, "a real skull drawn after the pass sees no ghost pose");
}
