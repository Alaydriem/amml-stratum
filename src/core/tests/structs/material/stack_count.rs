use stratum_core::structs::material::StackCount;

#[test]
fn stack_count_splits_counts() {
    assert_eq!(StackCount::of(3412, 64), StackCount { stacks: 53, rest: 20 });
    assert_eq!(StackCount::of(1536, 64), StackCount { stacks: 24, rest: 0 });
    assert_eq!(StackCount::of(42, 16), StackCount { stacks: 2, rest: 10 });
    assert_eq!(StackCount::of(0, 64), StackCount { stacks: 0, rest: 0 });
}

#[test]
fn stack_size_zero_counts_as_one() {
    assert_eq!(StackCount::of(5, 0), StackCount { stacks: 5, rest: 0 });
}
