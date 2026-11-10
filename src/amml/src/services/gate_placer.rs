use std::path::{Path, PathBuf};

use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::frame::FrameStatus;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::marker::MarkerTarget;

use super::SharedUi;

/// With the `gate` feature, on the game thread: when a world is first entered after enable, imports the first file
/// of the repo's `samples/` folder and places it as the active marker 2 blocks in front of the player, so the ghost
/// draws without a click. A marker is placed at the feet the last core tick saw, so the tick before the placement
/// sees the minimum corner of the structure in front as the feet. Once per enable.
pub struct GatePlacer {
    sample: Option<PathBuf>,
    imported: bool,
    // the structures the library held before the import
    known: Vec<StructureId>,
    // the imported structure, placed after the next core tick
    pending: Option<StructureId>,
    done: bool,
}

impl GatePlacer {
    /// Blocks between the feet of the player and the near side of the placed structure.
    pub const AHEAD: f32 = 2.0;
    const MARKER_NAME: &'static str = "Stratum gate";

    /// A placer for the first `.mcstructure` file, by name, in `folder`; it does nothing when there is none.
    pub fn new(folder: &Path) -> Self {
        Self { sample: Self::first_sample(folder), imported: false, known: Vec::new(), pending: None, done: false }
    }

    /// The repo's `samples/` folder, as the build saw it.
    pub fn samples_folder() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("samples")
    }

    /// Before the core's tick: the feet the tick should see, Some on the tick before the placement. `yaw` is the
    /// player's look angle around, in degrees.
    pub fn feet_for_tick(&mut self, status: &FrameStatus, yaw: Option<f32>, shared: &SharedUi) -> Option<[i32; 3]> {
        if self.done || !status.in_world || !status.has_player {
            return None;
        }
        let Some(sample) = self.sample.clone() else {
            curia::warn!("gate: no sample structure to place", {});
            self.done = true;
            return None;
        };
        let snapshot = shared.snapshot();
        if !self.imported {
            self.known = snapshot.unsorted.iter().map(|row| row.id).collect();
            shared.send(&mut vec![LibraryAction::ImportFiles(vec![sample.clone()])]);
            self.imported = true;
            curia::info!("gate: sample import sent", { path: sample.display().to_string() });
            return None;
        }
        let new = snapshot.unsorted.iter().find(|row| !self.known.contains(&row.id))?;
        self.pending = Some(new.id);
        self.done = true;
        let size = [new.size.x, new.size.y, new.size.z];
        Some(Self::origin_ahead(status.feet, yaw.unwrap_or(0.0), size))
    }

    /// After the core's tick: places the imported structure at the feet that tick saw.
    pub fn after_tick(&mut self, shared: &SharedUi) {
        let Some(id) = self.pending.take() else { return };
        let target = MarkerTarget::Structure(id);
        shared.send(&mut vec![LibraryAction::PlaceMarker { name: Self::MARKER_NAME.to_string(), target }]);
        curia::info!("gate: sample placed", { structure: id.0 });
    }

    /// The minimum corner, at the feet level, of a structure of `size` that starts `AHEAD` blocks in front of
    /// `feet`, along the main axis of the look direction `yaw`, and is centred across it, so the player and a camera
    /// behind the player stay outside it. The game's yaw 0 faces +z and 90 faces -x.
    pub fn origin_ahead(feet: [i32; 3], yaw: f32, size: [u32; 3]) -> [i32; 3] {
        let radians = yaw.to_radians();
        let (dx, dz) = (-radians.sin(), radians.cos());
        let ahead = Self::AHEAD as i32;
        let [sx, _, sz] = size.map(|side| i32::try_from(side.max(1)).unwrap_or(i32::MAX));
        // the near side is AHEAD blocks out on the facing axis; the structure grows away from the player
        let along =
            |feet: i32, side: i32, forward: bool| if forward { feet + ahead } else { feet - ahead - (side - 1) };
        if dx.abs() >= dz.abs() {
            [along(feet[0], sx, dx > 0.0), feet[1], feet[2] - sz / 2]
        } else {
            [feet[0] - sx / 2, feet[1], along(feet[2], sz, dz > 0.0)]
        }
    }

    fn first_sample(folder: &Path) -> Option<PathBuf> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
            .ok()?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|extension| extension == "mcstructure"))
            .collect();
        files.sort();
        files.into_iter().next().and_then(|path| std::path::absolute(&path).ok())
    }
}
