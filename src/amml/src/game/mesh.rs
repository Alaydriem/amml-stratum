use std::ffi::c_void;
use std::mem::MaybeUninit;

use crate::game::{MeshLayout2651, OffscreenCapture, ScreenContext, TextureVariant};
use crate::types::GhostCalls;

// mce::Mesh::~Mesh()
type DtorFn = unsafe extern "C" fn(*mut MeshLayout2651);
// bool mce::Mesh::isValid() const
type IsValidFn = unsafe extern "C" fn(*const MeshLayout2651) -> u8;
// void mce::Mesh::renderMesh(MeshContext&, MaterialPtr const&, std::variant<..TexturePtr..> const&, uint start,
// uint count, OffscreenCaptureDescription const&, IndexBufferContainer const*) const
type RenderFn = unsafe extern "C" fn(
    *const MeshLayout2651,
    *mut c_void,
    *const c_void,
    *const TextureVariant,
    u32,
    u32,
    *const OffscreenCapture,
    *const c_void,
);

/// An `mce::Mesh` the mod owns, made by `Tessellator::end` and destroyed through the game's `Mesh::~Mesh` on drop.
/// The buffer is a [`MeshLayout2651`], the larger mirror, so it holds the mesh of either build. The mesh bytes are
/// boxed, so they never move once the game has constructed them: the mod does not assume that a
/// game `Mesh` may be moved with a byte copy. Render thread only.
pub struct OwnedMesh {
    bytes: Box<MaybeUninit<MeshLayout2651>>,
    constructed: bool,
    dtor: usize,
    is_valid: usize,
    render: usize,
}

impl OwnedMesh {
    /// A buffer for the game to construct a mesh in; dropping it before `assume_init` calls nothing.
    pub(crate) fn uninit(calls: &GhostCalls) -> Self {
        Self {
            bytes: Box::new(MaybeUninit::uninit()),
            constructed: false,
            dtor: calls.mesh_dtor,
            is_valid: calls.mesh_is_valid,
            render: calls.mesh_render,
        }
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut MeshLayout2651 {
        self.bytes.as_mut_ptr()
    }

    /// Marks the buffer as a constructed mesh, which the drop then destroys.
    ///
    /// # Safety
    /// The game must have constructed an `mce::Mesh` in the buffer.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn assume_init(mut self) -> Self {
        self.constructed = true;
        self
    }

    /// `Mesh::isValid`: the mesh has vertices or uploaded buffers.
    #[allow(unsafe_code)]
    pub fn is_valid(&self) -> bool {
        // SAFETY: the entry is the address of `Mesh::isValid` with the signature of `IsValidFn`
        let is_valid: IsValidFn = unsafe { std::mem::transmute(self.is_valid) };
        // SAFETY: the buffer holds a constructed Mesh
        self.constructed && unsafe { is_valid(self.bytes.as_ptr()) } != 0
    }

    /// Draws the whole mesh with `material` (a `mce::MaterialPtr const*`) and `texture`, in the world matrix and the
    /// actor constants the caller set on `screen`. Render thread, inside the frame `screen` belongs to.
    ///
    /// # Safety
    /// `material` must be null or a live `mce::MaterialPtr`, and `texture` a variant whose texture holds live uses.
    #[allow(unsafe_code)]
    pub unsafe fn render(&self, screen: &ScreenContext, material: *const c_void, texture: &TextureVariant) {
        if !self.constructed || material.is_null() {
            return;
        }
        // SAFETY: the entry is the address of the first `Mesh::renderMesh` overload with the signature of `RenderFn`
        let render: RenderFn = unsafe { std::mem::transmute(self.render) };
        let capture = OffscreenCapture::new(0, [0; 4]);
        // SAFETY: a constructed Mesh, the live MeshContext of the frame, a live material, a texture variant and an
        // empty capture description that live for the call; start and count 0 draw every index; no index override
        unsafe {
            render(self.bytes.as_ptr(), screen.mesh_context(), material, texture, 0, 0, &capture, std::ptr::null())
        };
    }
}

impl Drop for OwnedMesh {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        if !self.constructed {
            return;
        }
        // SAFETY: the entry is the address of `Mesh::~Mesh` with the signature of `DtorFn`
        let dtor: DtorFn = unsafe { std::mem::transmute(self.dtor) };
        // SAFETY: the buffer holds a constructed Mesh, destroyed here once
        unsafe { dtor(self.bytes.as_mut_ptr()) };
    }
}
