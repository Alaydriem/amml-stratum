use super::{AssembleResult, CountResult, OrderResult};

// What the build worker made of a `BuildJob`.
#[derive(Debug)]
pub enum BuildAnswer {
    Assemble(AssembleResult),
    Count(CountResult),
    Order(OrderResult),
}
