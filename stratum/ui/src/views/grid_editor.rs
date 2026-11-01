use amml_i18n::text::{Text, UserText};
use amml_ui::state::DisabledReason;
use amml_ui::widget::{GridCellRef, GridPlacer, GridPlacerResponse};
use egui::{Id, Ui};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::layout::GridCell;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::snapshot::{LibrarySnapshot, StructureRow};

use crate::message::StratumMessage;

// The grid fallback of a big build: the pieces on a grid, as `GridPlacer` draws it. The grid covers the placed
// pieces plus one spare column and row, and always has a cell for every piece, up to the placer's largest side.
pub struct GridEditor;

impl GridEditor {
    // The columns and rows of the grid for `pieces`.
    pub fn size(pieces: &[StructureRow]) -> (u16, u16) {
        let side = GridPlacer::MAX_SIDE;
        let placed = pieces.iter().filter_map(|piece| piece.grid_cell);
        let (mut columns, mut rows) = placed.fold((0u16, 0u16), |(columns, rows), cell| {
            (columns.max(cell.column.saturating_add(1)), rows.max(cell.row.saturating_add(1)))
        });
        columns = columns.saturating_add(1).min(side);
        rows = rows.saturating_add(1).min(side);
        let count = u32::try_from(pieces.len()).unwrap_or(u32::MAX);
        while u32::from(columns) * u32::from(rows) < count && columns < side {
            columns += 1;
        }
        (columns, rows)
    }

    // The number a cell shows on the grid: from 1, along each row.
    pub fn cell_number(pieces: &[StructureRow], cell: GridCell) -> u64 {
        let (columns, _) = Self::size(pieces);
        u64::from(cell.row) * u64::from(columns) + u64::from(cell.column) + 1
    }

    // Draws the grid and pushes `SetGridCell` for each piece the player places or takes back.
    pub fn show(
        ui: &mut Ui,
        id: Id,
        snapshot: &LibrarySnapshot,
        pieces: &[StructureRow],
        out: &mut Vec<LibraryAction>,
    ) {
        let (columns, rows) = Self::size(pieces);
        let mut placer = GridPlacer::new(id, columns, rows);
        for piece in pieces {
            let label = Text::user(UserText::from_model(piece.name.clone()));
            let cell = piece.grid_cell.map(|cell| GridCellRef::new(cell.column, cell.row));
            placer = placer.piece(piece.id.0, label, cell);
        }
        let reason = DisabledReason::message(StratumMessage::ReadOnlyReason);
        placer = placer.enabled(!snapshot.library_read_only, reason);
        match placer.show(ui) {
            GridPlacerResponse::None => {}
            GridPlacerResponse::Placed { key, cell } => {
                out.push(LibraryAction::SetGridCell(StructureId(key), Some(GridCell::new(cell.column, cell.row))));
            }
            GridPlacerResponse::Unplaced { key } => out.push(LibraryAction::SetGridCell(StructureId(key), None)),
        }
    }
}
