use crate::support::FakeBuffer;
use alaydriem_stratum::game::TexturePtrLayout;

#[test]
fn texture_ptr_layout_is_two_shared_pointers() {
    assert_eq!(size_of::<TexturePtrLayout>(), 32);
    assert_eq!(TexturePtrLayout::VARIANT_INDEX, 1);
}

#[test]
fn texture_ptr_layout_reads_both_shared_pointers() {
    let mut buffer = FakeBuffer::new(32);
    buffer.put_u64(0, 0x10);
    buffer.put_u64(16, 0x30);

    let texture = buffer.view::<TexturePtrLayout>();

    assert_eq!(texture.client_texture().get().map(|value| value as *const _ as usize), Some(0x10));
    assert_eq!(texture.resource_location().get().map(|value| value as *const _ as usize), Some(0x30));
    assert_eq!(texture.client_texture().use_count(), 0);
}

#[test]
#[allow(unsafe_code)]
fn clone_ref_takes_one_use_of_each_pointer_and_release_gives_them_back() {
    use amml_msvc::RefCountBase;
    use std::sync::atomic::AtomicI32;
    // two control blocks the game holds 2 uses of; neither reaches zero, so the vtable is never read
    let client = RefCountBase { vtable: std::ptr::null(), uses: AtomicI32::new(2), weaks: AtomicI32::new(1) };
    let location = RefCountBase { vtable: std::ptr::null(), uses: AtomicI32::new(2), weaks: AtomicI32::new(1) };
    let mut buffer = FakeBuffer::new(32);
    buffer.put_u64(0, 0x10);
    buffer.put_u64(8, &client as *const RefCountBase as u64);
    buffer.put_u64(16, 0x30);
    buffer.put_u64(24, &location as *const RefCountBase as u64);
    let texture = buffer.view::<TexturePtrLayout>();

    // SAFETY: both pointers hold live uses of the control blocks above
    let mut copy = unsafe { texture.clone_ref() };

    assert_eq!(texture.client_texture().use_count(), 3);
    assert_eq!(texture.resource_location().use_count(), 3);
    assert_eq!(copy.client_texture().get().map(|value| value as *const _ as usize), Some(0x10));
    // SAFETY: the copy holds the two uses `clone_ref` took
    unsafe { copy.release() };
    assert_eq!(texture.client_texture().use_count(), 2);
    assert_eq!(texture.resource_location().use_count(), 2);
    assert!(copy.client_texture().get().is_none());
}
