// The filter over the markers list.
pub struct MarkerFilter;

impl MarkerFilter {
    // The world needs more markers than this before the sidebar shows the filter field.
    pub const MARKER_FILTER_THRESHOLD: usize = 8;

    // True for an empty or all-space filter; otherwise true when the trimmed filter is part of the marker name or the
    // name of its target, with no case difference.
    pub fn matches(name: &str, target_name: &str, filter: &str) -> bool {
        let filter = filter.trim();
        if filter.is_empty() {
            return true;
        }
        let filter = filter.to_lowercase();
        name.to_lowercase().contains(&filter) || target_name.to_lowercase().contains(&filter)
    }
}
