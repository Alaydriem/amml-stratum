use amml_msvc::{MsvcError, MsvcHeap, MsvcVec};

/// Edits a game `std::vector` in place: its elements as a slice, and growth on the game heap with the MSVC STL's
/// rule (the capacity grows by half, or to the size asked for when that is more), so the game's own code can go on
/// using and freeing the vector.
pub struct GameVec;

impl GameVec {
    /// The vector's elements.
    #[allow(unsafe_code)]
    pub fn as_mut_slice<T>(vec: &mut MsvcVec<T>) -> &mut [T] {
        let [first, _, _] = vec.raw_parts();
        if first.is_null() {
            return &mut [];
        }
        let len = vec.len();
        // SAFETY: a non-null vector holds `len` initialised elements from `first`, borrowed with the vector
        unsafe { std::slice::from_raw_parts_mut(first, len) }
    }

    /// Makes room for `total` elements; the elements stay as they were. The old block, if any, is freed once after
    /// the copy.
    ///
    /// # Safety
    /// The vector's block must have come from `heap` (or be null), and `heap` must be the game's allocator.
    #[allow(unsafe_code)]
    pub unsafe fn reserve<T: Copy>(heap: &MsvcHeap, vec: &mut MsvcVec<T>, total: usize) -> Result<(), MsvcError> {
        let [first, _, end] = vec.raw_parts();
        let len = vec.len();
        let capacity = if first.is_null() {
            0
        } else {
            // SAFETY: a non-null vector has `first <= end` inside one block
            unsafe { end.offset_from(first) as usize }
        };
        if total <= capacity {
            return Ok(());
        }
        let grown = capacity + capacity / 2;
        let new_capacity = if grown < total { total } else { grown };
        let bytes = new_capacity * size_of::<T>();
        // SAFETY: the caller guarantees the game's allocator
        let block = unsafe { heap.allocate(bytes) }.cast::<T>();
        if block.is_null() {
            return Err(MsvcError::AllocationFailed { bytes });
        }
        // SAFETY: the new block has room for `new_capacity >= len` elements and does not overlap the old one; the
        // old block came from `heap` for `capacity` elements and is not used again
        unsafe {
            if !first.is_null() {
                std::ptr::copy_nonoverlapping(first, block, len);
                heap.deallocate(first.cast(), capacity * size_of::<T>());
            }
            vec.set_raw_parts(block, block.add(len), block.add(new_capacity));
        }
        Ok(())
    }

    /// Appends `items`, growing the vector when its capacity is short.
    ///
    /// # Safety
    /// As [`GameVec::reserve`].
    #[allow(unsafe_code)]
    pub unsafe fn extend<T: Copy>(heap: &MsvcHeap, vec: &mut MsvcVec<T>, items: &[T]) -> Result<(), MsvcError> {
        if items.is_empty() {
            return Ok(());
        }
        let len = vec.len();
        // SAFETY: forwarded to the caller
        unsafe { Self::reserve(heap, vec, len + items.len()) }?;
        let [first, _, end] = vec.raw_parts();
        // SAFETY: the reserve left room for `len + items.len()` elements from `first`
        unsafe {
            std::ptr::copy_nonoverlapping(items.as_ptr(), first.add(len), items.len());
            vec.set_raw_parts(first, first.add(len + items.len()), end);
        }
        Ok(())
    }

    /// Sets the length to `len`: new elements are `value`, and a shorter length drops the elements past it.
    ///
    /// # Safety
    /// As [`GameVec::reserve`].
    #[allow(unsafe_code)]
    pub unsafe fn resize<T: Copy>(
        heap: &MsvcHeap,
        vec: &mut MsvcVec<T>,
        len: usize,
        value: T,
    ) -> Result<(), MsvcError> {
        let old = vec.len();
        if len > old {
            // SAFETY: forwarded to the caller
            unsafe { Self::reserve(heap, vec, len) }?;
        }
        let [first, _, end] = vec.raw_parts();
        if first.is_null() {
            return Ok(());
        }
        // SAFETY: the vector has room for `len` elements from `first`; the elements are Copy, so a shorter length
        // drops nothing that needs a destructor
        unsafe {
            for index in old..len {
                first.add(index).write(value);
            }
            vec.set_raw_parts(first, first.add(len), end);
        }
        Ok(())
    }
}
