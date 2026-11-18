use amml_i18n::text::Text;
use amml_ui::widget::{ButtonKind, TextButton};
use egui::Ui;
use stratum_core::structs::action::{BuildAction, LibraryAction};
use stratum_core::structs::build::BuildPhase;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::dialogs::BuildDialog;
use crate::message::StratumMessage;
use crate::state::{Dialog, LibraryUiState};

// The build buttons at the start of a structure's action row. Build, when a build of this structure is allowed and
// none runs; Pause or Resume and Stop while this structure is being built. Nothing otherwise.
pub struct BuildActions;

impl BuildActions {
    // Paints the buttons into the row `ui` is laying out. A click pushes its build action into `out`; Build also
    // opens the settings panel.
    pub fn show(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        structure: StructureId,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
    ) {
        let running = snapshot.build.as_ref().filter(|row| row.phase.is_running());
        match running {
            Some(row) if row.structure == structure => Self::running(ui, row.phase, out),
            Some(_) => {}
            None if snapshot.build_allowed == Some(structure) => Self::idle(ui, structure, state, out),
            None => {}
        }
    }

    fn idle(ui: &mut Ui, structure: StructureId, state: &mut LibraryUiState, out: &mut Vec<LibraryAction>) {
        let build = TextButton::new(ButtonKind::Primary, Text::message(StratumMessage::Build));
        if build.show(ui).clicked() {
            out.push(LibraryAction::Build(BuildAction::Open(structure)));
            state.dialog = Some(Dialog::Build(BuildDialog::new(structure)));
        }
    }

    fn running(ui: &mut Ui, phase: BuildPhase, out: &mut Vec<LibraryAction>) {
        let (label, action) = match phase {
            BuildPhase::Paused => (StratumMessage::BuildResume, BuildAction::Resume),
            _ => (StratumMessage::BuildPause, BuildAction::Pause),
        };
        if TextButton::new(ButtonKind::Secondary, Text::message(label)).show(ui).clicked() {
            out.push(LibraryAction::Build(action));
        }
        if TextButton::new(ButtonKind::Danger, Text::message(StratumMessage::BuildStop)).show(ui).clicked() {
            out.push(LibraryAction::Build(BuildAction::Stop));
        }
    }
}
