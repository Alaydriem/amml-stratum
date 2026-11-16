use amml_msvc::{MsvcHeap, MsvcVec};

use crate::game::{GameVec, MeshDataLayout};

/// Adds a second copy of a block's quads with the vertex order of each quad reversed, so a one-sided shape is seen
/// from both sides. Every per-vertex vector the tessellator filled for the block is copied; normals and tangents are
/// negated in x, y and z.
pub struct QuadMirror;

impl QuadMirror {
    const QUAD: usize = 4;

    /// Appends the reversed copy of the vertices from `first` on and returns how many vertices it added. `None`, with
    /// the mesh data as it was, when the vertices are not whole quads, the mesh has an index buffer, a per-vertex
    /// vector holds a count other than one entry per vertex (or none), or the game heap cannot grow a vector.
    ///
    /// # Safety
    /// Every vector of `data` must have come from `heap` (or be null), and `heap` must be the game's allocator.
    #[allow(unsafe_code)]
    pub unsafe fn append(heap: &MsvcHeap, data: &mut MeshDataLayout, first: usize) -> Option<usize> {
        let last = data.positions.len();
        let added = last.saturating_sub(first);
        if !added.is_multiple_of(Self::QUAD) || !data.indices.is_empty() {
            return None;
        }
        let whole = Self::whole(&data.positions, last)
            && Self::whole(&data.colors, last)
            && Self::whole(&data.normals, last)
            && Self::whole(&data.tangents, last)
            && Self::whole(&data.bone_id0s, last)
            && Self::whole(&data.pbr_texture_indices, last)
            && Self::whole(&data.mers, last)
            && Self::whole(&data.geo_type, last)
            && data.texture_uvs.iter().all(|uvs| Self::whole(uvs, last));
        if !whole {
            return None;
        }
        if added == 0 {
            return Some(0);
        }
        // SAFETY: forwarded to the caller
        let room = unsafe {
            Self::reserve(heap, &mut data.positions, last, added)
                && Self::reserve(heap, &mut data.colors, last, added)
                && Self::reserve(heap, &mut data.normals, last, added)
                && Self::reserve(heap, &mut data.tangents, last, added)
                && Self::reserve(heap, &mut data.bone_id0s, last, added)
                && Self::reserve(heap, &mut data.pbr_texture_indices, last, added)
                && Self::reserve(heap, &mut data.mers, last, added)
                && Self::reserve(heap, &mut data.geo_type, last, added)
                && data.texture_uvs.iter_mut().all(|uvs| Self::reserve(heap, uvs, last, added))
        };
        if !room {
            return None;
        }
        // SAFETY: the room is reserved for every vector that is copied
        unsafe {
            Self::copy(heap, &mut data.positions, first, last, |vertex| vertex);
            Self::copy(heap, &mut data.colors, first, last, |vertex| vertex);
            Self::copy(heap, &mut data.normals, first, last, |[x, y, z, w]| [-x, -y, -z, w]);
            Self::copy(heap, &mut data.tangents, first, last, |[x, y, z, w]| [-x, -y, -z, w]);
            Self::copy(heap, &mut data.bone_id0s, first, last, |vertex| vertex);
            Self::copy(heap, &mut data.pbr_texture_indices, first, last, |vertex| vertex);
            Self::copy(heap, &mut data.mers, first, last, |vertex| vertex);
            Self::copy(heap, &mut data.geo_type, first, last, |vertex| vertex);
            for uvs in &mut data.texture_uvs {
                Self::copy(heap, uvs, first, last, |vertex| vertex);
            }
        }
        Some(added)
    }

    // a vector with one entry per vertex, or none
    fn whole<T>(vec: &MsvcVec<T>, last: usize) -> bool {
        vec.is_empty() || vec.len() == last
    }

    // makes room for `added` more elements in a vector that holds one element for each of the `last` vertices; true
    // for an empty vector, which is left out
    #[allow(unsafe_code)]
    unsafe fn reserve<T: Copy>(heap: &MsvcHeap, vec: &mut MsvcVec<T>, last: usize, added: usize) -> bool {
        if vec.is_empty() {
            return true;
        }
        // SAFETY: forwarded to the caller
        unsafe { GameVec::reserve(heap, vec, last + added) }.is_ok()
    }

    // appends the quads of `first..last`, each reversed, to a vector that holds `last` elements
    #[allow(unsafe_code)]
    unsafe fn copy<T: Copy>(heap: &MsvcHeap, vec: &mut MsvcVec<T>, first: usize, last: usize, map: impl Fn(T) -> T) {
        if vec.len() != last {
            return;
        }
        for base in (first..last).step_by(Self::QUAD) {
            let source = vec.as_slice();
            let reversed = [map(source[base + 3]), map(source[base + 2]), map(source[base + 1]), map(source[base])];
            // SAFETY: the room was reserved, so the append does not grow the vector
            let _ = unsafe { GameVec::extend(heap, vec, &reversed) };
        }
    }
}
