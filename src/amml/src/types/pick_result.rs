// One answer of the structure file picker. `status` is the first failing COM call's HRESULT, or 0 (S_OK); a cancel
// is `CANCELLED`. `dropped` counts chosen items whose path could not be read, `invalid` the paths that were not
// valid UTF-16.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickResult {
    pub status: i32,
    pub paths: Vec<String>,
    pub dropped: u32,
    pub invalid: u32,
}

impl PickResult {
    // HRESULT_FROM_WIN32(ERROR_CANCELLED)
    pub const CANCELLED: i32 = 0x8007_04C7_u32 as i32;

    // the answer of a dialog that was not shown or was closed: a cancel with no paths
    pub fn cancelled() -> Self {
        Self { status: Self::CANCELLED, paths: Vec::new(), dropped: 0, invalid: 0 }
    }

    // a failure other than a cancel
    pub fn failed(&self) -> bool {
        self.status < 0 && self.status != Self::CANCELLED
    }
}
