// A count split into full stacks and the items left over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackCount {
    pub stacks: u64,
    pub rest: u64,
}

impl StackCount {
    // A stack size of 0 counts as 1.
    pub fn of(count: u64, stack_size: u32) -> StackCount {
        let size = u64::from(stack_size.max(1));
        StackCount { stacks: count / size, rest: count % size }
    }
}
