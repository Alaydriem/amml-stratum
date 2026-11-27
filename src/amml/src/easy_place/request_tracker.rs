use amml_api::{ApiError, RequestId, RequestOutcome};
use stratum_core::structs::easy_place::SwapAnswer;

/// The inventory request of the hold: the id the loader gave the last swap or pick, and the answer that the probe's
/// swap answer reads. A request the loader refused is answered as refused at once; the loader ends every request it
/// took with `InventoryRequestDone`, so no timer runs here.
pub struct RequestTracker {
    awaited: Option<RequestId>,
    answer: SwapAnswer,
}

impl RequestTracker {
    pub fn new() -> Self {
        Self { awaited: None, answer: SwapAnswer::None }
    }

    pub fn answer(&self) -> SwapAnswer {
        self.answer
    }

    pub fn awaited(&self) -> Option<RequestId> {
        self.awaited
    }

    /// What the loader said to a swap or pick call.
    pub fn sent(&mut self, result: Result<RequestId, ApiError>) {
        match result {
            Ok(id) => {
                self.awaited = Some(id);
                self.answer = SwapAnswer::None;
            }
            Err(error) => {
                curia::warn!("easy place inventory request not taken", { error: format!("{error:?}") });
                self.awaited = None;
                self.answer = SwapAnswer::Refused;
            }
        }
    }

    /// An `InventoryRequestDone`; one for another request (another mod's, or an older one) is ignored.
    pub fn done(&mut self, id: RequestId, outcome: RequestOutcome) {
        if self.awaited != Some(id) {
            return;
        }
        self.awaited = None;
        self.answer = if outcome == RequestOutcome::Done { SwapAnswer::Accepted } else { SwapAnswer::Refused };
        if outcome != RequestOutcome::Done {
            curia::info!("easy place inventory request ended", { outcome: format!("{outcome:?}") });
        }
    }

    pub fn reset(&mut self) {
        self.awaited = None;
        self.answer = SwapAnswer::None;
    }
}

impl Default for RequestTracker {
    fn default() -> Self {
        Self::new()
    }
}
