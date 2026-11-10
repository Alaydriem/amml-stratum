use crate::game::MatrixStackRefLayout;

// MatrixStack::MatrixStackRef::~MatrixStackRef(): pops the pushed matrix
type RefDtorFn = unsafe extern "C" fn(*mut MatrixStackRefLayout);

/// A matrix pushed on the camera's world matrix stack; the drop pops it through the game's `~MatrixStackRef`.
/// Render thread, inside the frame it was pushed in.
pub struct WorldMatrixRef {
    layout: MatrixStackRefLayout,
    dtor: usize,
}

impl WorldMatrixRef {
    pub(crate) fn new(layout: MatrixStackRefLayout, dtor: usize) -> Self {
        Self { layout, dtor }
    }

    /// The pushed matrix, a `glm::mat4` stored by columns.
    pub(crate) fn matrix(&self) -> *mut [[f32; 4]; 4] {
        self.layout.matrix
    }
}

impl Drop for WorldMatrixRef {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: the entry is the address of `~MatrixStackRef` with the signature of `RefDtorFn`
        let dtor: RefDtorFn = unsafe { std::mem::transmute(self.dtor) };
        // SAFETY: the ref is the one `MatrixStack::push` returned, still pushed, and destroyed here once
        unsafe { dtor(&mut self.layout) };
    }
}
