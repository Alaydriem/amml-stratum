use stratum_core::structs::structure::SparseLayer;

#[test]
fn unsorted_cells_are_sorted() {
    let layer = SparseLayer::new(vec![9, 2, 5], vec![90, 20, 50]);

    let pairs: Vec<(u32, u16)> = layer.iter().collect();
    assert_eq!(pairs, [(2, 20), (5, 50), (9, 90)]);
    assert_eq!(layer.get(5), Some(50));
}

#[test]
fn a_repeated_cell_keeps_its_first_pair() {
    let layer = SparseLayer::new(vec![4, 1, 4], vec![40, 10, 41]);

    let pairs: Vec<(u32, u16)> = layer.iter().collect();
    assert_eq!(pairs, [(1, 10), (4, 40)]);
    assert_eq!(layer.len(), 2);
}

#[test]
fn extra_items_in_the_longer_vector_are_dropped() {
    let more_cells = SparseLayer::new(vec![1, 2, 3], vec![10]);
    let more_blocks = SparseLayer::new(vec![1], vec![10, 20, 30]);

    assert_eq!(more_cells.iter().collect::<Vec<_>>(), [(1, 10)]);
    assert_eq!(more_blocks.iter().collect::<Vec<_>>(), [(1, 10)]);
    assert_eq!(more_cells.heap_bytes(), 6);
}

#[test]
fn get_on_a_cell_without_a_block_is_none() {
    let layer = SparseLayer::new(vec![3, 7], vec![30, 70]);

    assert_eq!(layer.get(0), None);
    assert_eq!(layer.get(5), None);
    assert_eq!(layer.get(8), None);
    assert_eq!(layer.get(usize::MAX), None);
    assert_eq!(SparseLayer::default().get(0), None);
}
