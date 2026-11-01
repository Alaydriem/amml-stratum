// The server's answer to the last swap or pick the shim sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapAnswer {
    // No answer yet.
    None,
    Accepted,
    Refused,
}
