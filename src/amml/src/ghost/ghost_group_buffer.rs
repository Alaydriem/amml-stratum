use amml_msvc::{MsvcHeap, MsvcVec};
use stratum_core::structs::ghost::{GhostJob, GhostJobMode};

use crate::game::{GameVec, MeshDataLayout, OwnedMeshData};
use crate::types::{GhostCalls, GroupStep, PrimitiveMode};

/// The one mini ghost group being built: the vertices of the Append jobs applied so far, in one game `MeshData` whose
/// vectors grow on the game heap, and the job identity they belong to (slot, group build serial, next index); and
/// per slot the group build serial and bytes of the group it finished last.
///
/// Bound: one buffer at a time, its size held by core's group cell limit; the per-slot records are sized with the
/// slot table.
pub struct GhostGroupBuffer {
    calls: GhostCalls,
    heap: MsvcHeap,
    data: Option<OwnedMeshData>,
    slot: u32,
    serial: u32,
    next: u32,
    finished: Vec<Option<u32>>,
    finished_bytes: Vec<u64>,
}

impl GhostGroupBuffer {
    pub fn new(calls: &GhostCalls) -> Self {
        Self {
            calls: *calls,
            heap: MsvcHeap::new(calls.heap_new, calls.heap_delete),
            data: None,
            slot: 0,
            serial: 0,
            next: 0,
            finished: Vec::new(),
            finished_bytes: Vec::new(),
        }
    }

    /// Sized with the slot table; holds nothing after.
    pub fn resize(&mut self, slots: usize) {
        self.data = None;
        self.finished.clear();
        self.finished.resize(slots, None);
        self.finished_bytes.clear();
        self.finished_bytes.resize(slots, 0);
    }

    pub fn step(&self, job: &GhostJob) -> GroupStep {
        if self.data.is_some() && job.slot == self.slot && job.group_serial == self.serial {
            return match job.group_index.cmp(&self.next) {
                std::cmp::Ordering::Equal => GroupStep::Apply,
                std::cmp::Ordering::Less => GroupStep::Repeat,
                std::cmp::Ordering::Greater => GroupStep::Ignore,
            };
        }
        let finished = self.finished.get(job.slot as usize).copied().flatten();
        if job.mode == GhostJobMode::Finish && finished == Some(job.group_serial) {
            return GroupStep::RepeatFinish;
        }
        if job.group_index == 0 { GroupStep::Apply } else { GroupStep::Ignore }
    }

    /// An empty buffer for the job's slot and group build, in place of any other; the job is at index 0.
    pub fn start(&mut self, job: &GhostJob) {
        self.data = None;
        let mut data = OwnedMeshData::new(&self.calls);
        data.layout_mut().mode = PrimitiveMode::TriangleList as u8;
        self.data = Some(data);
        self.slot = job.slot;
        self.serial = job.group_serial;
        self.next = 0;
    }

    /// Adds every vertex field of `source` to the buffer and moves on to the next index. False, with the buffer as
    /// it was, when `source` has indices of its own or a field that does not cover every vertex; false also when
    /// the game heap gives no block, and the caller then drops the buffer.
    #[allow(unsafe_code)]
    pub fn add(&mut self, source: &MeshDataLayout) -> bool {
        let Some(data) = self.data.as_mut() else { return false };
        let to = data.layout_mut();
        let total = to.positions.len() + source.positions.len();
        if !source.indices.is_empty() || !Self::fields_cover(to, source, total) {
            return false;
        }
        if !source.positions.is_empty() {
            let unfilled = to.positions.is_empty();
            for corner in 0..3 {
                to.aabb[0][corner] =
                    if unfilled { source.aabb[0][corner] } else { to.aabb[0][corner].min(source.aabb[0][corner]) };
                to.aabb[1][corner] =
                    if unfilled { source.aabb[1][corner] } else { to.aabb[1][corner].max(source.aabb[1][corner]) };
            }
            for corner in 0..2 {
                to.uv_aabb[0][corner] = if unfilled {
                    source.uv_aabb[0][corner]
                } else {
                    to.uv_aabb[0][corner].min(source.uv_aabb[0][corner])
                };
                to.uv_aabb[1][corner] = if unfilled {
                    source.uv_aabb[1][corner]
                } else {
                    to.uv_aabb[1][corner].max(source.uv_aabb[1][corner])
                };
            }
        }
        let heap = self.heap;
        // SAFETY: the buffer's vectors were made by the game's MeshData constructor and grow only here, on the game
        // heap the game's destructor frees them to
        let grown = unsafe {
            GameVec::extend(&heap, &mut to.positions, source.positions.as_slice()).is_ok()
                && GameVec::extend(&heap, &mut to.normals, source.normals.as_slice()).is_ok()
                && GameVec::extend(&heap, &mut to.tangents, source.tangents.as_slice()).is_ok()
                && GameVec::extend(&heap, &mut to.colors, source.colors.as_slice()).is_ok()
                && GameVec::extend(&heap, &mut to.bone_id0s, source.bone_id0s.as_slice()).is_ok()
                && to
                    .texture_uvs
                    .iter_mut()
                    .zip(&source.texture_uvs)
                    .all(|(to, from)| GameVec::extend(&heap, to, from.as_slice()).is_ok())
                && GameVec::extend(&heap, &mut to.pbr_texture_indices, source.pbr_texture_indices.as_slice()).is_ok()
                && GameVec::extend(&heap, &mut to.mers, source.mers.as_slice()).is_ok()
                && GameVec::extend(&heap, &mut to.geo_type, source.geo_type.as_slice()).is_ok()
        };
        if !grown {
            return false;
        }
        for (enabled, more) in to.field_enabled.iter_mut().zip(source.field_enabled) {
            *enabled = *enabled || more;
        }
        self.next += 1;
        true
    }

    /// The buffer's vertices for an upload; `finish` drops the buffer.
    pub fn data_mut(&mut self) -> Option<&mut OwnedMeshData> {
        self.data.as_mut()
    }

    pub fn vertices(&self) -> u64 {
        self.data.as_ref().map_or(0, |data| data.layout().positions.len() as u64)
    }

    /// Drops the buffer and records the slot's finished group with what its mesh holds.
    pub fn finish(&mut self, slot: u32, serial: u32, bytes: u64) {
        self.data = None;
        let index = slot as usize;
        if index < self.finished.len() {
            self.finished[index] = Some(serial);
            self.finished_bytes[index] = bytes;
        }
    }

    pub fn finished_bytes(&self, slot: u32) -> u64 {
        self.finished_bytes.get(slot as usize).copied().unwrap_or(0)
    }

    /// Drops the buffer.
    pub fn drop_build(&mut self) {
        self.data = None;
    }

    /// Drops the buffer if it is for `slot`, and the slot's finished record.
    pub fn free(&mut self, slot: u32) {
        if self.data.is_some() && self.slot == slot {
            self.data = None;
        }
        let index = slot as usize;
        if index < self.finished.len() {
            self.finished[index] = None;
            self.finished_bytes[index] = 0;
        }
    }

    /// Drops the buffer and every finished record.
    pub fn clear(&mut self) {
        self.data = None;
        self.finished.fill(None);
        self.finished_bytes.fill(0);
    }

    // a field filled by some parts and not by others would give later vertices another vertex's value
    fn fields_cover(to: &MeshDataLayout, from: &MeshDataLayout, total: usize) -> bool {
        Self::covers(&to.normals, &from.normals, total)
            && Self::covers(&to.tangents, &from.tangents, total)
            && Self::covers(&to.colors, &from.colors, total)
            && Self::covers(&to.bone_id0s, &from.bone_id0s, total)
            && Self::covers(&to.pbr_texture_indices, &from.pbr_texture_indices, total)
            && Self::covers(&to.mers, &from.mers, total)
            && Self::covers(&to.geo_type, &from.geo_type, total)
            && to.texture_uvs.iter().zip(&from.texture_uvs).all(|(to, from)| Self::covers(to, from, total))
    }

    // the buffer and the new vertices together hold the field for none or for all of `total` vertices
    fn covers<T>(had: &MsvcVec<T>, more: &MsvcVec<T>, total: usize) -> bool {
        let held = had.len() + more.len();
        held == 0 || held == total
    }
}
