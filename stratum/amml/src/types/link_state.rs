// Where the registration with the mod manager stands for the current enable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    // not enabled, or the last try failed
    Off,
    // enable found no manager; the first tick tries once more
    Waiting,
    Attached,
}
