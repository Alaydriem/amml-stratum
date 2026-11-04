// The request serials of one preview view: the newest it sent and the newest that came back for it. A frame whose
// serial is past the newest sent was asked for by an earlier view on the same service and is not an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreviewSerials {
    sent: u64,
    answered: u64,
}

impl PreviewSerials {
    pub fn new() -> Self {
        Self::default()
    }

    // The serial for the next request.
    pub fn next(&self) -> u64 {
        self.sent + 1
    }

    // Records that the request with `serial` was sent.
    pub fn sent(&mut self, serial: u64) {
        self.sent = self.sent.max(serial);
    }

    // Records a frame that came back and returns whether it answers this view.
    pub fn answer(&mut self, serial: u64) -> bool {
        if serial > self.sent {
            return false;
        }
        self.answered = self.answered.max(serial);
        true
    }

    // True while the newest request sent has no answer.
    pub fn waiting(&self) -> bool {
        self.answered < self.sent
    }
}
