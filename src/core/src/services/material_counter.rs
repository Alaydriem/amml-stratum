use std::collections::HashMap;

use crate::services::ItemRules;
use crate::structs::material::{ItemId, ItemYield, MaterialLine, MaterialList};
use crate::structs::structure::{BlockLayer, Structure};

// Builds the list of items a structure needs.
pub struct MaterialCounter;

impl MaterialCounter {
    // Counts the primary layer only: waterlogging in the secondary layer adds nothing.
    pub fn count(rules: &ItemRules, structure: &Structure) -> MaterialList {
        // One pass over the blocks into a count per palette entry.
        let mut per_entry = vec![0u64; structure.palette.len()];
        for index in structure.primary.as_slice() {
            if *index == BlockLayer::NONE {
                continue;
            }
            if let Some(slot) = per_entry.get_mut(usize::from(*index)) {
                *slot += 1;
            }
        }

        // Then one rule lookup per palette entry.
        let mut totals: HashMap<ItemId, u64> = HashMap::new();
        for (block, count) in structure.palette.iter().zip(per_entry) {
            if count == 0 {
                continue;
            }
            match rules.items_for(block) {
                ItemYield::None => {}
                ItemYield::One(item, each) => Self::add(&mut totals, item, count, each),
                ItemYield::Two(first, first_each, second, second_each) => {
                    Self::add(&mut totals, first, count, first_each);
                    Self::add(&mut totals, second, count, second_each);
                }
            }
        }

        let lines = totals
            .into_iter()
            .map(|(item, count)| {
                let stack_size = rules.stack_size(&item);
                MaterialLine { item, count, stack_size }
            })
            .collect();
        MaterialList::new(lines)
    }

    fn add(totals: &mut HashMap<ItemId, u64>, item: ItemId, blocks: u64, each: u32) {
        let entry = totals.entry(item).or_insert(0);
        *entry = entry.saturating_add(blocks.saturating_mul(u64::from(each)));
    }
}
