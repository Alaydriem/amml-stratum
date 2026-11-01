/// What a mini ghost group job asks for, against the group the renderer holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupStep {
    /// The next job of the buffer's group build, or index 0 of a new one.
    Apply,
    /// A job of the buffer's group build that was applied already: answered with the buffer's bytes.
    Repeat,
    /// The Finish of the slot's last finished group: answered with that group's bytes.
    RepeatFinish,
    /// Not answered; core hands it out again.
    Ignore,
}
