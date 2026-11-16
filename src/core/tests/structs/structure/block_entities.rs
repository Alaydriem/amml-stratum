use stratum_core::structs::structure::BlockEntities;

#[test]
fn equal_nbt_in_two_cells_is_one_entry() {
    let mut entities = BlockEntities::new();
    assert!(entities.insert(7, vec![1, 2, 3]));
    assert!(entities.insert(2, vec![1, 2, 3]));

    assert_eq!(entities.len(), 2);
    assert_eq!(entities.entry_of(7), entities.entry_of(2));
    let entry = entities.entry_of(7).unwrap();
    assert_eq!(entities.entry(entry), Some([1u8, 2, 3].as_slice()));
    assert_eq!(entities.entry(entry + 1), None);
    // Cells come out in rising order.
    let cells: Vec<u32> = entities.cells().map(|(cell, _)| cell).collect();
    assert_eq!(cells, vec![2, 7]);
}

#[test]
fn block_entities_past_the_bound_are_dropped_and_counted() {
    let mut entities = BlockEntities::new();
    let total = BlockEntities::MAX_ENTRIES as u32 + 3;
    for cell in 0..total {
        entities.insert(cell, cell.to_le_bytes().to_vec());
    }

    assert_eq!(entities.len(), BlockEntities::MAX_ENTRIES);
    assert_eq!(entities.dropped(), 3);
    for cell in total - 3..total {
        assert_eq!(entities.entry_of(cell), None);
    }
    assert!(entities.entry_of(0).is_some());
    // Equal NBT still maps once the bound is reached.
    assert!(entities.insert(total, 0u32.to_le_bytes().to_vec()));
    assert_eq!(entities.dropped(), 3);
}
