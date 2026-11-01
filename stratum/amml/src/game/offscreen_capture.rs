use amml_msvc::MsvcVariant;

/// The game's `OffscreenCaptureDescription`, a `std::variant<monostate, AtlasTileCaptureDescription (8),
/// UIThumbnailMeshOffscreenCaptureDescription (32), UIStructureVolumeOffscreenCaptureDescription (32)>` (40 bytes:
/// the 32-byte storage, then the index byte). A ghost draw passes the empty one: index 0, `monostate`.
pub type OffscreenCapture = MsvcVariant<[u64; 4]>;

const _: () = assert!(size_of::<OffscreenCapture>() == 40);
