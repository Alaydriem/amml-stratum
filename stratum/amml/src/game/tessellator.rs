use std::ffi::c_void;
use std::ptr::NonNull;

use crate::game::{MeshDataLayout, MeshLayout2651, OpenBuild, OwnedMesh, TessellatorLayout};
use crate::types::{GhostCalls, MeshBuild, PrimitiveMode};

// void Tessellator::begin(DebugContextCallback, mce::PrimitiveMode, int reserved, bool): this, the empty callback
// struct in one byte, the mode, the vertex count to reserve, then a flag the ghost passes as false
type BeginFn = unsafe extern "C" fn(*mut c_void, u8, u8, i32, u8);
// mce::Mesh Tessellator::end(UploadMode, std::string_view, SupplementaryFieldAutoGenerationMode): this, the hidden
// Mesh buffer, the upload mode, a pointer to the debug name view, then the 2-byte mode; the buffer comes back in RAX
type EndFn = unsafe extern "C" fn(*mut c_void, *mut MeshLayout2651, i32, *const [usize; 2], u16) -> *mut MeshLayout2651;
// void Tessellator::clear()
type ClearFn = unsafe extern "C" fn(*mut c_void);

/// The game's `Tessellator`, the one a `ScreenContext` holds. Render thread only.
pub struct Tessellator {
    ptr: NonNull<TessellatorLayout>,
    calls: GhostCalls,
}

impl Tessellator {
    /// `Tessellator::UploadMode::Buffered`: the mesh is uploaded on its first draw.
    const UPLOAD_BUFFERED: i32 = 0;
    /// `Tessellator::UploadMode::Never`: the mesh is never uploaded, for a mesh that is dropped at once.
    const UPLOAD_NEVER: i32 = 2;
    /// `SupplementaryFieldAutoGenerationMode::None`.
    const NO_SUPPLEMENTARY_FIELDS: u16 = 0;

    /// Wraps `ptr`; `None` for a null pointer.
    ///
    /// # Safety
    /// `ptr` must be null or a live `Tessellator` of the build `calls` was resolved for, laid out as
    /// `TessellatorLayout`, and must stay live while the wrapper is used.
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(ptr: *mut c_void, calls: &GhostCalls) -> Option<Self> {
        NonNull::new(ptr.cast::<TessellatorLayout>()).map(|ptr| Self { ptr, calls: *calls })
    }

    /// The wrapped pointer.
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr().cast()
    }

    /// Whether a begin is open: `mTessellating`.
    #[allow(unsafe_code)]
    pub fn is_tessellating(&self) -> bool {
        // SAFETY: `from_raw` guarantees a live Tessellator laid out as the mirror
        unsafe { (*self.ptr.as_ptr()).tessellating }
    }

    /// Begins a quad list, the mode the block tessellator writes in. The game does nothing while a begin is open.
    pub fn begin_quads(&mut self, reserve: i32) {
        self.begin(PrimitiveMode::QuadList, reserve);
    }

    /// Begins a triangle list, the mode of a mesh with its own index buffer.
    pub fn begin_triangles(&mut self, reserve: i32) {
        self.begin(PrimitiveMode::TriangleList, reserve);
    }

    #[allow(unsafe_code)]
    fn begin(&mut self, mode: PrimitiveMode, reserve: i32) {
        // SAFETY: the entry is the address of `Tessellator::begin` with the signature of `BeginFn`
        let begin: BeginFn = unsafe { std::mem::transmute(self.calls.tessellator_begin) };
        // SAFETY: `from_raw` guarantees a live Tessellator
        unsafe { begin(self.as_ptr(), 0, mode as u8, reserve, 0) };
    }

    /// Ends the open begin into a mesh that is uploaded on its first draw; `None`, with no call, when no begin is
    /// open.
    pub fn end(&mut self, debug_name: &str) -> Option<OwnedMesh> {
        self.end_with(Self::UPLOAD_BUFFERED, debug_name)
    }

    /// Ends the open begin into a mesh that is never uploaded, for one the caller drops at once; `None`, with no
    /// call, when no begin is open.
    pub fn end_unuploaded(&mut self, debug_name: &str) -> Option<OwnedMesh> {
        self.end_with(Self::UPLOAD_NEVER, debug_name)
    }

    #[allow(unsafe_code)]
    fn end_with(&mut self, upload: i32, debug_name: &str) -> Option<OwnedMesh> {
        if !self.is_tessellating() {
            return None;
        }
        // SAFETY: the entry is the address of `Tessellator::end` with the signature of `EndFn`
        let end: EndFn = unsafe { std::mem::transmute(self.calls.tessellator_end) };
        let name = [debug_name.as_ptr() as usize, debug_name.len()];
        let mut mesh = OwnedMesh::uninit(&self.calls);
        // SAFETY: a live Tessellator with an open begin; the buffer is 632 bytes, 8-aligned, and `end` constructs
        // the mesh in it; the name view lives for the call
        unsafe { end(self.as_ptr(), mesh.as_mut_ptr(), upload, &name, Self::NO_SUPPLEMENTARY_FIELDS) };
        // SAFETY: `end` constructed a Mesh in the buffer
        Some(unsafe { mesh.assume_init() })
    }

    /// Drops what the open begin collected; the game also closes the begin.
    #[allow(unsafe_code)]
    pub fn clear(&mut self) {
        // SAFETY: the entry is the address of `Tessellator::clear` with the signature of `ClearFn`
        let clear: ClearFn = unsafe { std::mem::transmute(self.calls.tessellator_clear) };
        // SAFETY: `from_raw` guarantees a live Tessellator
        unsafe { clear(self.as_ptr()) };
    }

    /// The mesh data a build fills: `mMeshData`.
    #[allow(unsafe_code)]
    pub fn mesh_data(&mut self) -> &mut MeshDataLayout {
        // SAFETY: `from_raw` guarantees a live Tessellator laid out as the mirror; the borrow ends with the wrapper's
        unsafe { &mut (*self.ptr.as_ptr()).mesh_data }
    }

    /// Sets `mCount`, the vertex count `end` uploads, for mesh data moved in by hand.
    #[allow(unsafe_code)]
    pub fn set_count(&mut self, count: u32) {
        // SAFETY: `from_raw` guarantees a live Tessellator laid out as the mirror
        unsafe { (*self.ptr.as_ptr()).count = count };
    }

    /// The vertices the tessellator holds: the length of `mMeshData.mPositions`.
    #[allow(unsafe_code)]
    pub fn vertex_count(&self) -> usize {
        // SAFETY: `from_raw` guarantees a live Tessellator laid out as the mirror
        unsafe { (*self.ptr.as_ptr()).mesh_data.positions.len() }
    }

    /// Moves every position the tessellator holds by minus `origin`, so the mesh is relative to it.
    #[allow(unsafe_code)]
    pub fn shift_positions(&mut self, origin: [f32; 3]) {
        // SAFETY: `from_raw` guarantees a live Tessellator laid out as the mirror
        let [first, last, _] = unsafe { (*self.ptr.as_ptr()).mesh_data.positions.raw_parts() };
        if first.is_null() {
            return;
        }
        // SAFETY: the game's vector holds `last - first` initialised positions from `first`, and nothing else
        // touches them on the render thread during the call
        let positions = unsafe { std::slice::from_raw_parts_mut(first, last.offset_from(first) as usize) };
        for position in positions {
            for axis in 0..3 {
                position[axis] -= origin[axis];
            }
        }
    }

    /// Begins a quad list, runs `fill` and always ends it, also when `fill` panics (the mesh of a panicked build
    /// is dropped). `None`, with nothing begun, when a begin is already open.
    pub fn build_quads(
        &mut self,
        reserve: i32,
        debug_name: &str,
        fill: impl FnOnce(&mut Tessellator) -> bool,
    ) -> Option<MeshBuild> {
        if self.is_tessellating() {
            return None;
        }
        self.begin_quads(reserve);
        let mut open = OpenBuild::new(self, debug_name);
        let filled = fill(open.tessellator());
        let vertices = open.tessellator().vertex_count();
        let mesh = open.finish();
        Some(MeshBuild { filled, vertices, mesh })
    }
}
