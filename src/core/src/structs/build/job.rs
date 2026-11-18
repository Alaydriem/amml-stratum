use super::{AssembleRequest, CountRequest, OrderRequest};

// A piece of work for the build worker.
#[derive(Debug, Clone)]
pub enum BuildJob {
    Assemble(AssembleRequest),
    Count(CountRequest),
    Order(OrderRequest),
}
