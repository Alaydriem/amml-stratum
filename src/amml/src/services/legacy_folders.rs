use std::path::{Path, PathBuf};

use amml_sdk::ModDirs;

use crate::types::{CopyFailure, CopyOutcome, FolderCopy, LegacyCopy, SkipReason};

// The one-time copy of the folders the LeviLamina build of stratum used, `<game>/mods/alaydriem-stratum/{config,data}`,
// into the AMML folders. Both hosts put AMML mods in `<game>/mods/amml/@<namespace>/<name>/`, so the old folders are at
// `<mod_dir>/../../../alaydriem-stratum`. A folder is copied only when its new folder is missing or empty and the old one
// exists. The copy is all or nothing: it is made in a temporary folder beside the new one, which takes the new
// folder's name only when every file copied. On a failure the temporary folder is removed and the new folder stays as
// it was, so the next enable tries again. The old folders are never changed
pub struct LegacyFolders;

impl LegacyFolders {
    pub const OLD_MOD_FOLDER: &'static str = "alaydriem-stratum";
    // the temporary folder of `data` is `data.legacy-tmp`, beside it
    pub const TEMPORARY_SUFFIX: &'static str = ".legacy-tmp";

    // the config folder, then the data folder
    pub fn copy_once(mod_dirs: &ModDirs) -> LegacyCopy {
        let old = mod_dirs
            .mod_dir
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(|mods| mods.join(Self::OLD_MOD_FOLDER));
        let folders = [("config", &mod_dirs.config_dir), ("data", &mod_dirs.data_dir)]
            .into_iter()
            .map(|(name, to)| {
                let from = old.as_ref().map_or_else(PathBuf::new, |old| old.join(name));
                let outcome = Self::copy_folder(&from, to);
                FolderCopy { name, from, to: to.clone(), outcome }
            })
            .collect();
        LegacyCopy { folders }
    }

    fn copy_folder(from: &Path, to: &Path) -> CopyOutcome {
        if from.as_os_str().is_empty() || !from.is_dir() {
            return CopyOutcome::Skipped(SkipReason::NoOldFolder);
        }
        if std::fs::read_dir(to).is_ok_and(|mut entries| entries.next().is_some()) {
            return CopyOutcome::Skipped(SkipReason::NewFolderHasData);
        }
        let mut temporary = to.as_os_str().to_owned();
        temporary.push(Self::TEMPORARY_SUFFIX);
        let temporary = PathBuf::from(temporary);
        let mut failures = Vec::new();
        let mut files = 0;
        // a temporary folder that is there already is what is left of a copy the game ended
        let fresh = Self::remove_tree(&temporary).and_then(|()| std::fs::create_dir_all(&temporary));
        match fresh {
            Ok(()) => Self::copy_tree(from, &temporary, &mut files, &mut failures),
            Err(error) => failures.push(CopyFailure { path: temporary.clone(), error: error.to_string() }),
        }
        if failures.is_empty()
            && let Err(error) = Self::take_name(&temporary, to)
        {
            failures.push(CopyFailure { path: to.to_path_buf(), error: error.to_string() });
        }
        if failures.is_empty() {
            return CopyOutcome::Copied { files };
        }
        if let Err(error) = Self::remove_tree(&temporary) {
            failures.push(CopyFailure { path: temporary, error: error.to_string() });
        }
        CopyOutcome::Failed { failures }
    }

    // gives the full copy at `temporary` the name `to`; an empty folder at `to` is removed first, and one that holds
    // data stops the rename
    fn take_name(temporary: &Path, to: &Path) -> std::io::Result<()> {
        if to.is_dir() {
            std::fs::remove_dir(to)?;
        }
        std::fs::rename(temporary, to)
    }

    // removes `path` and everything under it, read-only files too; a missing `path` is not an error
    fn remove_tree(path: &Path) -> std::io::Result<()> {
        let kind = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata.file_type(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if kind.is_dir() {
            for entry in std::fs::read_dir(path)? {
                Self::remove_tree(&entry?.path())?;
            }
            return std::fs::remove_dir(path);
        }
        let mut permissions = std::fs::metadata(path)?.permissions();
        if permissions.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            std::fs::set_permissions(path, permissions)?;
        }
        std::fs::remove_file(path)
    }

    // copies every file and folder under `from` into `to`, which exists; a failure is noted and the rest is copied
    fn copy_tree(from: &Path, to: &Path, files: &mut usize, failures: &mut Vec<CopyFailure>) {
        let entries = match std::fs::read_dir(from) {
            Ok(entries) => entries,
            Err(error) => {
                failures.push(CopyFailure { path: from.to_path_buf(), error: error.to_string() });
                return;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    failures.push(CopyFailure { path: from.to_path_buf(), error: error.to_string() });
                    continue;
                }
            };
            let (source, target) = (entry.path(), to.join(entry.file_name()));
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                match std::fs::create_dir_all(&target) {
                    Ok(()) => Self::copy_tree(&source, &target, files, failures),
                    Err(error) => failures.push(CopyFailure { path: source, error: error.to_string() }),
                }
                continue;
            }
            match std::fs::copy(&source, &target) {
                Ok(_) => *files += 1,
                Err(error) => failures.push(CopyFailure { path: source, error: error.to_string() }),
            }
        }
    }
}
