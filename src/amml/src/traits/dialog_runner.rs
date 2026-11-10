use crate::types::PickResult;

// Shows the structure file picker and blocks until it closes. Runs on the picker's own thread; a WM_CLOSE posted
// to the windows of that thread closes it as a cancel
pub trait DialogRunner: Send + Sync + 'static {
    fn pick(&self) -> PickResult;
}
