use super::settings_entry::BuildSettingsEntry;

// The content of `build.hcl`, oldest use first.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct BuildSettingsFile {
    #[serde(default)]
    pub(crate) settings: Vec<BuildSettingsEntry>,
    // The worlds owed a game rule restore, oldest first. A file saved before this field existed has none.
    #[serde(default)]
    pub(crate) owed: Vec<String>,
}
