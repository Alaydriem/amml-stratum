use amml_api::{ApiError, RequestId};

/// What carrying out one use command gave, for the driver and the core once their locks are taken again: the spot of
/// a placement the game did not make, and the loader's answer to a swap or pick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UseOutcome {
    pub not_sent: Option<[i32; 3]>,
    pub request: Option<Result<RequestId, ApiError>>,
}
