/// An 8-byte aligned, zeroed byte block that a test fills at the header offsets of a game struct and then reads
/// through a layout mirror.
pub struct FakeBuffer {
    words: Vec<u64>,
}

impl FakeBuffer {
    pub fn new(len: usize) -> Self {
        FakeBuffer { words: vec![0; len.div_ceil(8)] }
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        let len = self.words.len() * 8;
        // SAFETY: the u64 block is `len` initialised bytes, and u8 has no alignment requirement.
        unsafe { std::slice::from_raw_parts_mut(self.words.as_mut_ptr().cast::<u8>(), len) }
    }

    pub fn put_u8(&mut self, offset: usize, value: u8) {
        self.bytes_mut()[offset] = value;
    }

    pub fn put_u16(&mut self, offset: usize, value: u16) {
        self.bytes_mut()[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn put_u32(&mut self, offset: usize, value: u32) {
        self.bytes_mut()[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub fn put_u64(&mut self, offset: usize, value: u64) {
        self.bytes_mut()[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    pub fn put_f32(&mut self, offset: usize, value: f32) {
        self.put_u32(offset, value.to_bits());
    }

    /// The block's address, to pass to a wrapper as a game object.
    pub fn as_mut_ptr(&mut self) -> *mut std::ffi::c_void {
        self.words.as_mut_ptr().cast()
    }

    /// Reads the block as `T`.
    pub fn view<T>(&self) -> &T {
        assert!(size_of::<T>() <= self.words.len() * 8, "the fake buffer is smaller than the mirror");
        assert!(align_of::<T>() <= 8, "the mirror needs more than 8-byte alignment");
        // SAFETY: the block is large enough and 8-byte aligned (checked above); the mirrors are plain data for
        // which a zeroed or test-filled bit pattern is valid.
        unsafe { &*self.words.as_ptr().cast::<T>() }
    }
}
