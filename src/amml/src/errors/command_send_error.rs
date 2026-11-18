/// A command was not sent, so nothing of it is left in the game.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CommandSendError {
    #[error("the game heap is not there")]
    NoHeap,
    #[error("the client has no packet sender")]
    NoSender,
    #[error("the game heap could not make the command text")]
    NoText,
    #[error("the game made no command request packet")]
    NotMade,
    #[error("the packet sender did not send the packet")]
    NotSent,
}
