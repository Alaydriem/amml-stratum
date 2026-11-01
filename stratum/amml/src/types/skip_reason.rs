// Why an old LeviLamina folder was not copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    // there is no old folder
    NoOldFolder,
    // the new folder already holds data
    NewFolderHasData,
}

impl SkipReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoOldFolder => "no old folder",
            Self::NewFolderHasData => "the new folder already holds data",
        }
    }
}
