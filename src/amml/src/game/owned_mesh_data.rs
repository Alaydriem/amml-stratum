use crate::game::MeshDataLayout;
use crate::types::GhostCalls;

// void mce::MeshData::clear(): resets the mode, the bounds and the field flags; the vectors must be empty
type ClearFn = unsafe extern "C" fn(*mut MeshDataLayout);
// mce::MeshData::~MeshData()
type DtorFn = unsafe extern "C" fn(*mut MeshDataLayout);
// mce::MeshData& mce::MeshData::operator=(mce::MeshData&&)
type MoveAssignFn = unsafe extern "C" fn(*mut MeshDataLayout, *mut MeshDataLayout) -> *mut MeshDataLayout;

/// An `mce::MeshData` the mod owns: built as the game's constructor builds one (zeroed bytes, then the game's
/// `MeshData::clear`; 26.60 has no constructor function) and destroyed with the game's destructor on drop, so every vector in it lives on the game heap. The bytes are boxed and never move. Render thread only.
pub struct OwnedMeshData {
    data: Box<MeshDataLayout>,
    dtor: usize,
    move_assign: usize,
}

impl OwnedMeshData {
    /// An empty mesh data.
    #[allow(unsafe_code)]
    pub fn new(calls: &GhostCalls) -> Self {
        // SAFETY: zeroed bytes are a valid MeshDataLayout: null vectors, zero floats and false flags
        let mut data = unsafe { Box::<MeshDataLayout>::new_zeroed().assume_init() };
        // SAFETY: the entry is the address of `MeshData::clear` with the signature of `ClearFn`
        let clear: ClearFn = unsafe { std::mem::transmute(calls.mesh_data_clear) };
        // SAFETY: a MeshData-sized, aligned buffer of zero bytes (empty vectors), cleared here once
        unsafe { clear(&mut *data) };
        Self { data, dtor: calls.mesh_data_dtor, move_assign: calls.mesh_data_move_assign }
    }

    pub fn layout(&self) -> &MeshDataLayout {
        &self.data
    }

    pub fn layout_mut(&mut self) -> &mut MeshDataLayout {
        &mut self.data
    }

    /// Moves every vector into `target` with the game's move assignment; this mesh data is left empty.
    ///
    /// # Safety
    /// `target` must be a live `mce::MeshData` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn move_into(&mut self, target: *mut MeshDataLayout) {
        // SAFETY: the entry is the address of the move assignment with the signature of `MoveAssignFn`
        let move_assign: MoveAssignFn = unsafe { std::mem::transmute(self.move_assign) };
        // SAFETY: the caller passes a live target; the source is this live mesh data
        unsafe { move_assign(target, &mut *self.data) };
    }
}

impl Drop for OwnedMeshData {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: the entry is the address of the destructor with the signature of `DtorFn`
        let dtor: DtorFn = unsafe { std::mem::transmute(self.dtor) };
        // SAFETY: the mesh data was constructed in `new` and is destroyed here once
        unsafe { dtor(&mut *self.data) };
    }
}
