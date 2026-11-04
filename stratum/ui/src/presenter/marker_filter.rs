// The filter over the markers list.
pub struct MarkerFilter;

impl MarkerFilter {
    // The world needs more markers than this before the sidebar shows the filter field.
    pub const MARKER_FILTER_THRESHOLD: usize = 8;

    // True for an empty or all-space filter; otherwise true when the trimmed filter is part of the marker name, the
    // name of its target or the name of the structure it shows now, with no case difference.
    pub fn matches(name: &str, target_name: &str, part_name: Option<&str>, filter: &str) -> bool {
        let filter = filter.trim();
        if filter.is_empty() {
            return true;
        }
        let filter = filter.to_lowercase();
        name.to_lowercase().contains(&filter)
            || target_name.to_lowercase().contains(&filter)
            || part_name.is_some_and(|part| part.to_lowercase().contains(&filter))
    }
}
