use std::path::PathBuf;

use super::CopyOutcome;

// One old LeviLamina folder and the AMML folder it is copied into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderCopy {
    // `config` or `data`
    pub name: &'static str,
    pub from: PathBuf,
    pub to: PathBuf,
    pub outcome: CopyOutcome,
}

impl FolderCopy {
    // the one log line of this folder, and whether it is a warning
    pub fn line(&self) -> (bool, String) {
        let (from, to) = (self.from.display(), self.to.display());
        match &self.outcome {
            CopyOutcome::Copied { files } => {
                (false, format!("old {} folder copied: {from} to {to} ({files} files)", self.name))
            }
            CopyOutcome::Skipped(reason) => {
                (false, format!("old {} folder skipped: {from} ({})", self.name, reason.as_str()))
            }
            CopyOutcome::Failed { failures } => {
                let failed: Vec<String> =
                    failures.iter().map(|failure| format!("{}: {}", failure.path.display(), failure.error)).collect();
                (
                    true,
                    format!(
                        "old {} folder failed: {from} to {to} ({} failed: {}); nothing was kept, the next start tries                          again",
                        self.name,
                        failures.len(),
                        failed.join("; ")
                    ),
                )
            }
        }
    }
}
