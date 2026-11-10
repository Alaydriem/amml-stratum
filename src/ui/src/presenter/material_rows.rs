use stratum_core::structs::material::{MaterialList, StackCount};

use super::MaterialRow;

// Builds the rows of the materials table from a material list.
pub struct MaterialRows;

impl MaterialRows {
    // One row per line, in the order of the list. A filter keeps the rows whose name contains it, ignoring case.
    pub fn build(list: &MaterialList, filter: &str) -> Vec<MaterialRow> {
        let needle = filter.trim().to_lowercase();
        list.lines()
            .iter()
            .map(|line| MaterialRow {
                name: line.item.readable(),
                count: line.count,
                stacks: StackCount::of(line.count, line.stack_size),
                stack_size: line.stack_size,
            })
            .filter(|row| needle.is_empty() || row.name.to_lowercase().contains(&needle))
            .collect()
    }
}
