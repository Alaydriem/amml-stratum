use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use alaydriem_stratum::services::LegacyFolders;
use alaydriem_stratum::types::{CopyOutcome, SkipReason};
use amml_sdk::ModDirs;
use tempfile::TempDir;

// A game folder with the AMML folders of stratum at mods/amml/@alaydriem/stratum and the old LeviLamina folders at
// mods/alaydriem-stratum.
struct Game {
    root: TempDir,
}

impl Game {
    fn new() -> Self {
        Self { root: TempDir::new().unwrap() }
    }

    fn dirs(&self) -> ModDirs {
        let mod_dir = self.root.path().join("mods").join("amml").join("@alaydriem").join("stratum");
        ModDirs { config_dir: mod_dir.join("config"), data_dir: mod_dir.join("data"), mod_dir }
    }

    fn old(&self, name: &str) -> PathBuf {
        self.root.path().join("mods").join("alaydriem-stratum").join(name)
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    // an old config folder with config.hcl, and an old data folder with a library and a structure in a subfolder
    fn with_old_folders(self) -> Self {
        Self::write(&self.old("config").join("config.hcl"), "ghost { opacity = 0.5 }");
        Self::write(&self.old("data").join("library.hcl"), "project \"Castle\" {}");
        Self::write(&self.old("data").join("structures").join("tower.mcstructure"), "tower bytes");
        self
    }

    // every file under `root`, by relative path, with its bytes
    fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut files = BTreeMap::new();
        let mut folders = vec![root.to_path_buf()];
        while let Some(folder) = folders.pop() {
            let Ok(entries) = std::fs::read_dir(&folder) else { continue };
            for entry in entries {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    folders.push(path);
                } else {
                    files.insert(path.strip_prefix(root).unwrap().to_path_buf(), std::fs::read(&path).unwrap());
                }
            }
        }
        files
    }

    // the folder a copy of the old `name` folder is made in before it takes the new folder's name
    fn temporary(&self, name: &str) -> PathBuf {
        self.dirs().mod_dir.join(format!("{name}{}", LegacyFolders::TEMPORARY_SUFFIX))
    }

    fn old_tree(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        Self::tree(&self.root.path().join("mods").join("alaydriem-stratum"))
    }
}

#[test]
fn old_folders_and_no_new_folders_are_copied_once() {
    let game = Game::new().with_old_folders();
    let before = game.old_tree();

    let first = LegacyFolders::copy_once(&game.dirs());
    let second = LegacyFolders::copy_once(&game.dirs());

    let outcomes: Vec<_> = first.folders.iter().map(|folder| (folder.name, folder.outcome.clone())).collect();
    assert_eq!(outcomes, [("config", CopyOutcome::Copied { files: 1 }), ("data", CopyOutcome::Copied { files: 2 })]);
    assert_eq!(Game::tree(&game.dirs().config_dir), Game::tree(&game.old("config")));
    assert_eq!(Game::tree(&game.dirs().data_dir), Game::tree(&game.old("data")));
    assert!(second.folders.iter().all(|folder| folder.outcome == CopyOutcome::Skipped(SkipReason::NewFolderHasData)));
    assert_eq!(game.old_tree(), before);
}

#[test]
fn an_empty_new_folder_is_filled() {
    let game = Game::new().with_old_folders();
    std::fs::create_dir_all(game.dirs().data_dir).unwrap();

    let copy = LegacyFolders::copy_once(&game.dirs());

    assert_eq!(copy.folders[1].outcome, CopyOutcome::Copied { files: 2 });
    assert_eq!(Game::tree(&game.dirs().data_dir).len(), 2);
}

#[test]
fn a_new_folder_with_data_is_left_as_it_is() {
    let game = Game::new().with_old_folders();
    let before = game.old_tree();
    Game::write(&game.dirs().data_dir.join("library.hcl"), "project \"New\" {}");

    let copy = LegacyFolders::copy_once(&game.dirs());

    assert_eq!(copy.folders[1].outcome, CopyOutcome::Skipped(SkipReason::NewFolderHasData));
    assert_eq!(std::fs::read_to_string(game.dirs().data_dir.join("library.hcl")).unwrap(), "project \"New\" {}");
    assert_eq!(Game::tree(&game.dirs().data_dir).len(), 1);
    assert_eq!(copy.folders[0].outcome, CopyOutcome::Copied { files: 1 }, "the config folder is decided on its own");
    assert_eq!(game.old_tree(), before);
}

#[test]
fn no_old_folder_skips_the_copy() {
    let game = Game::new();

    let copy = LegacyFolders::copy_once(&game.dirs());

    assert!(copy.folders.iter().all(|folder| folder.outcome == CopyOutcome::Skipped(SkipReason::NoOldFolder)));
    assert!(!game.dirs().data_dir.exists(), "no new folder is made for nothing");
    assert_eq!(copy.lines().len(), 2);
    assert!(copy.lines().iter().all(|(warn, line)| !warn && line.contains("skipped")));
}

#[test]
fn a_file_that_cannot_be_read_fails_the_folder_and_leaves_no_new_folder() {
    let game = Game::new().with_old_folders();
    let locked = game.old("data").join("library.hcl");
    let before = game.old_tree();
    // no share mode: until the handle drops, no other open of the file succeeds
    let handle = OpenOptions::new().read(true).share_mode(0).open(&locked).unwrap();

    let copy = LegacyFolders::copy_once(&game.dirs());
    drop(handle);

    let CopyOutcome::Failed { failures } = &copy.folders[1].outcome else {
        panic!("the data folder did not fail: {:?}", copy.folders[1].outcome)
    };
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].path, locked);
    assert!(!game.dirs().data_dir.exists(), "no part of the data folder was kept");
    assert!(!game.temporary("data").exists(), "the temporary folder was removed");
    let (warn, line) = &copy.lines()[1];
    assert!(*warn && line.contains("failed") && line.contains("library.hcl"), "{line}");
    assert_eq!(copy.folders[0].outcome, CopyOutcome::Copied { files: 1 }, "the config folder is decided on its own");
    assert_eq!(game.old_tree(), before);
}

#[test]
fn a_failed_copy_into_an_empty_new_folder_leaves_it_empty() {
    let game = Game::new().with_old_folders();
    std::fs::create_dir_all(game.dirs().data_dir).unwrap();
    let handle = OpenOptions::new().read(true).share_mode(0).open(game.old("data").join("library.hcl")).unwrap();

    let copy = LegacyFolders::copy_once(&game.dirs());
    drop(handle);

    assert!(matches!(copy.folders[1].outcome, CopyOutcome::Failed { .. }), "{:?}", copy.folders[1].outcome);
    assert!(Game::tree(&game.dirs().data_dir).is_empty());
    assert!(!game.temporary("data").exists());
}

#[test]
fn the_next_copy_after_a_failed_one_copies_every_file() {
    let game = Game::new().with_old_folders();
    let before = game.old_tree();
    let handle = OpenOptions::new().read(true).share_mode(0).open(game.old("data").join("library.hcl")).unwrap();
    LegacyFolders::copy_once(&game.dirs());
    drop(handle);

    let copy = LegacyFolders::copy_once(&game.dirs());

    assert_eq!(copy.folders[1].outcome, CopyOutcome::Copied { files: 2 });
    assert_eq!(Game::tree(&game.dirs().data_dir), Game::tree(&game.old("data")));
    assert_eq!(copy.folders[0].outcome, CopyOutcome::Skipped(SkipReason::NewFolderHasData));
    assert_eq!(game.old_tree(), before);
}

#[test]
fn a_temporary_folder_left_by_an_ended_game_is_replaced() {
    let game = Game::new().with_old_folders();
    Game::write(&game.temporary("data").join("structures").join("half.mcstructure"), "half");

    let copy = LegacyFolders::copy_once(&game.dirs());

    assert_eq!(copy.folders[1].outcome, CopyOutcome::Copied { files: 2 });
    assert_eq!(Game::tree(&game.dirs().data_dir), Game::tree(&game.old("data")));
    assert!(!game.temporary("data").exists());
}

#[test]
fn a_read_only_file_is_copied_and_the_old_one_stays_read_only() {
    let game = Game::new().with_old_folders();
    let file = game.old("config").join("config.hcl");
    let mut permissions = std::fs::metadata(&file).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&file, permissions).unwrap();
    let before = game.old_tree();

    let copy = LegacyFolders::copy_once(&game.dirs());

    assert_eq!(copy.folders[0].outcome, CopyOutcome::Copied { files: 1 });
    assert!(std::fs::metadata(&file).unwrap().permissions().readonly());
    assert_eq!(game.old_tree(), before);
    // the temporary folder cannot remove a read-only file
    for path in [file, game.dirs().config_dir.join("config.hcl")] {
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        std::fs::set_permissions(&path, permissions).unwrap();
    }
}

#[test]
fn a_failed_copy_removes_the_read_only_files_it_copied() {
    let game = Game::new().with_old_folders();
    let file = game.old("config").join("config.hcl");
    let mut permissions = std::fs::metadata(&file).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&file, permissions).unwrap();
    Game::write(&game.old("config").join("keys.hcl"), "keys {}");
    let handle = OpenOptions::new().read(true).share_mode(0).open(game.old("config").join("keys.hcl")).unwrap();

    let copy = LegacyFolders::copy_once(&game.dirs());
    drop(handle);

    assert!(matches!(copy.folders[0].outcome, CopyOutcome::Failed { .. }), "{:?}", copy.folders[0].outcome);
    assert!(!game.temporary("config").exists(), "the read-only copy did not stop the removal");
    assert!(!game.dirs().config_dir.exists());
    let mut permissions = std::fs::metadata(&file).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&file, permissions).unwrap();
}

#[test]
fn the_legacy_folder_is_found_from_a_nested_mod_folder() {
    let game = Game::new().with_old_folders();
    let dirs = game.dirs();

    assert_eq!(dirs.mod_dir, game.root.path().join("mods").join("amml").join("@alaydriem").join("stratum"));
    let copy = LegacyFolders::copy_once(&dirs);

    assert_eq!(copy.folders[0].from, game.old("config"));
    assert_eq!(copy.folders[1].from, game.old("data"));
}
