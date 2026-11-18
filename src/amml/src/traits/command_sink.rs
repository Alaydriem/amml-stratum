// Where the build's commands go: the game's command request packet
pub trait CommandSink {
    // sends one command as typed in the chat; false when it was not sent
    fn send(&mut self, text: &str) -> bool;
}
