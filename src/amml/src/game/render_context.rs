use std::ffi::c_void;
use std::mem::offset_of;
use std::ptr::NonNull;

use amml_game::game::MatrixStackLayout;
use amml_game::{CameraLayout, RawObject};
use amml_msvc::MemberCall;

use crate::game::{MatrixStackRefLayout, MeshContextLayout, MeshContextOffset, WorldMatrixRef};
use crate::types::GhostCalls;

// static void ActorShaderManager::setEntityConstants(ActorConstants&, RenderContext&, Color const& tileLight,
// Vec2 const& tileLightUV, glm::vec4 const& blockLight, Color const& overlay, Color const& change,
// Color const& change2, Color const& glint, Vec2 const& glintUVScale, Vec4 const& uvAnim, float uvOffset1,
// float uvOffset2, float uvRot1, float uvRot2)
type SetEntityConstantsFn = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *const [f32; 4],
    *const [f32; 2],
    *const [f32; 4],
    *const [f32; 4],
    *const [f32; 4],
    *const [f32; 4],
    *const [f32; 4],
    *const [f32; 2],
    *const [f32; 4],
    f32,
    f32,
    f32,
    f32,
);

/// The game's `ScreenContext` of one render call: its tessellator, its `MeshContext` base and, through that, the
/// camera's world matrix stack and the actor constants. Render thread, inside the frame it belongs to.
pub struct ScreenContext {
    ptr: NonNull<c_void>,
    calls: GhostCalls,
}

impl ScreenContext {
    /// Where the world matrix stack sits in `mce::Camera`: after the view matrix stack, before the projection one.
    const WORLD_STACK: usize = offset_of!(CameraLayout, projection_stack) - size_of::<MatrixStackLayout>();

    /// Wraps `ptr`; `None` for a null pointer.
    ///
    /// # Safety
    /// `ptr` must be null or the live `ScreenContext` of the running render call, of the build `calls` was
    /// resolved for, and must stay live while the wrapper is used.
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(ptr: *mut c_void, calls: &GhostCalls) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self { ptr, calls: *calls })
    }

    /// The `mce::MeshContext&` base that `Mesh::renderMesh` takes.
    pub fn mesh_context(&self) -> *mut c_void {
        MeshContextOffset::from_screen(self.ptr.as_ptr() as usize) as *mut c_void
    }

    #[allow(unsafe_code)]
    fn mesh_fields(&self) -> &MeshContextLayout {
        // SAFETY: `from_raw` guarantees a live ScreenContext, whose MeshContext base the mirror lays out
        unsafe { &*self.mesh_context().cast::<MeshContextLayout>() }
    }

    /// The `Tessellator&` of the context; `None` when it is null.
    #[allow(unsafe_code)]
    pub fn tessellator(&self) -> Option<*mut c_void> {
        // SAFETY: `from_raw` guarantees a live ScreenContext, and the entry is the offset of its `tessellator`
        // reference, stored as a pointer
        unsafe { RawObject::pointer(self.ptr.as_ptr(), self.calls.screen_tessellator) }
    }

    /// Pushes a copy of the world matrix times a translation by `offset` on the camera's world matrix stack, as
    /// `glm::translate` does, and marks the stack dirty. The drop of the result pops it. `None` when the camera is
    /// null.
    pub fn push_world_translation(&self, offset: [f32; 3]) -> Option<WorldMatrixRef> {
        self.push_world_matrix(offset, 1.0)
    }

    /// Pushes a copy of the world matrix times `translate(offset) * scale(scale)` on the camera's world matrix stack
    /// and marks the stack dirty: a mesh vertex `p` lands at `offset + scale * p`. The drop of the result pops it.
    /// `None` when the camera is null.
    #[allow(unsafe_code)]
    pub fn push_world_matrix(&self, offset: [f32; 3], scale: f32) -> Option<WorldMatrixRef> {
        let camera = self.mesh_fields().camera;
        if camera.is_null() {
            return None;
        }
        let stack = camera.cast::<u8>().wrapping_add(Self::WORLD_STACK).cast::<c_void>();
        // SAFETY: the entry is the address of `MatrixStack::push(bool isSortOrigin)`, a member that returns its
        // 16-byte ref through the hidden pointer; `stack` is the live camera's world matrix stack
        let pushed: MatrixStackRefLayout =
            unsafe { MemberCall::call_sret1(self.calls.matrix_stack_push as *const c_void, stack, 1u8) };
        let reference = WorldMatrixRef::new(pushed, self.calls.matrix_stack_ref_dtor);
        let matrix = reference.matrix();
        if !matrix.is_null() {
            // SAFETY: `push` gave the new top matrix of the stack, a glm::mat4 stored by columns
            let columns = unsafe { &mut *matrix };
            let moved: [f32; 4] = std::array::from_fn(|row| {
                columns[0][row] * offset[0] + columns[1][row] * offset[1] + columns[2][row] * offset[2]
            });
            for (value, add) in columns[3].iter_mut().zip(moved) {
                *value += add;
            }
            for column in columns.iter_mut().take(3) {
                for value in column.iter_mut() {
                    *value *= scale;
                }
            }
            // SAFETY: the stack is live and laid out as the mirror; `_isDirty` tells the game the top changed
            unsafe { (*stack.cast::<MatrixStackLayout>()).is_dirty = true };
        }
        Some(reference)
    }

    /// The camera's projection times its view matrix, each the top of its stack, by columns as glm stores them;
    /// `None` when the camera is null or a stack is empty.
    #[allow(unsafe_code)]
    pub fn clip_matrix(&self) -> Option<[[f32; 4]; 4]> {
        let camera = self.mesh_fields().camera.cast::<CameraLayout>();
        if camera.is_null() {
            return None;
        }
        // SAFETY: the live camera of the frame, laid out as the mirror whose anchor the calls checked; the top of a
        // stack is a glm::mat4 the game keeps for the frame
        let (projection, view) = unsafe {
            let camera = &*camera;
            (*camera.projection_stack.stack.back()?, *camera.view_stack.stack.back()?)
        };
        Some(std::array::from_fn(|column| {
            std::array::from_fn(|row| (0..4).map(|k| projection[k][row] * view[column][k]).sum())
        }))
    }

    /// Writes the fixed actor constants of a ghost draw: white tile light and block light, no overlay, no change
    /// colours, no glint, UV animation offset 0 and scale 1. Without them a draw takes the last actor's light.
    #[allow(unsafe_code)]
    pub fn set_ghost_constants(&self) {
        let fields = self.mesh_fields();
        if fields.constant_buffers.is_null() || fields.render_context.is_null() {
            return;
        }
        let entity = fields.constant_buffers.cast::<u8>().wrapping_add(self.calls.entity_constant_buffer as usize);
        let white = [1.0f32; 4];
        let none = [0.0f32; 4];
        let one = [1.0f32; 2];
        let uv_anim = [0.0f32, 0.0, 1.0, 1.0];
        // SAFETY: the entry is the address of the static `ActorShaderManager::setEntityConstants`
        let set: SetEntityConstantsFn = unsafe { std::mem::transmute(self.calls.set_entity_constants) };
        // SAFETY: the frame's live entity constant buffer and render context, and values that live for the call
        unsafe {
            set(
                entity.cast(),
                fields.render_context,
                &white,
                &one,
                &white,
                &none,
                &none,
                &none,
                &none,
                &one,
                &uv_anim,
                0.0,
                0.0,
                0.0,
                0.0,
            )
        };
    }
}
