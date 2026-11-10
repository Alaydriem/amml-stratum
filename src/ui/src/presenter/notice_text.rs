use amml_i18n::text::{Args, Text, UserText};
use amml_ui::widget::ToastTone;
use stratum_core::errors::{LibraryError, MarkerError};
use stratum_core::structs::import::ImportFailure;
use stratum_core::structs::snapshot::ImportNotice;

use super::NameCheck;
use crate::message::StratumMessage;

// The words and tone of one import notice.
pub struct NoticeText;

impl NoticeText {
    const BYTES_PER_MB: u64 = 1_048_576;

    // A name that is too long is told with the limit alone: a reason inside another message cannot fill `{max}`.
    pub fn of(notice: &ImportNotice) -> (ToastTone, Text) {
        match notice {
            ImportNotice::ImportRefused { error: LibraryError::NameTooLong, .. }
            | ImportNotice::LibraryRefused(LibraryError::NameTooLong)
            | ImportNotice::MarkerRefused(MarkerError::NameTooLong) => (ToastTone::Danger, NameCheck::too_long()),
            ImportNotice::ImportFailed { file, reason } => (ToastTone::Danger, Self::failed(file, reason)),
            ImportNotice::ImportRefused { file, error } => {
                let args = Self::file(file).message("reason", Self::library_reason(error));
                (ToastTone::Danger, Text::format(StratumMessage::NoticeImportRefused, args))
            }
            ImportNotice::ImportQueueFull => (ToastTone::Warning, Text::message(StratumMessage::NoticeQueueFull)),
            ImportNotice::InboxFileKept { file } => {
                (ToastTone::Warning, Text::format(StratumMessage::NoticeInboxKept, Self::file(file)))
            }
            ImportNotice::MaterialsBusy => (ToastTone::Warning, Text::message(StratumMessage::NoticeMaterialsBusy)),
            ImportNotice::GhostCut => (ToastTone::Warning, Text::message(StratumMessage::NoticeGhostCut)),
            ImportNotice::GhostRefused => (ToastTone::Warning, Text::message(StratumMessage::NoticeGhostRefused)),
            ImportNotice::NotInWorld => (ToastTone::Warning, Text::message(StratumMessage::NotInWorldReason)),
            ImportNotice::MarkerRefused(error) => {
                let args = Args::new().message("reason", Self::marker_reason(error));
                (ToastTone::Danger, Text::format(StratumMessage::NoticeMarkerRefused, args))
            }
            ImportNotice::LibraryRefused(error) => {
                let args = Args::new().message("reason", Self::library_reason(error));
                (ToastTone::Danger, Text::format(StratumMessage::NoticeLibraryRefused, args))
            }
            ImportNotice::MaterialsFailed(_) => (ToastTone::Danger, Text::message(StratumMessage::MaterialsFailed)),
            ImportNotice::LibraryUnavailable => {
                (ToastTone::Warning, Text::message(StratumMessage::LibraryReadOnlyTitle))
            }
            ImportNotice::MarkersUnavailable => {
                (ToastTone::Warning, Text::message(StratumMessage::MarkersReadOnlyTitle))
            }
        }
    }

    fn file(file: &str) -> Args {
        Args::new().user("file", UserText::from_model(file.to_owned()))
    }

    fn failed(file: &str, reason: &ImportFailure) -> Text {
        match reason {
            ImportFailure::TooBig { bytes } => {
                let megabytes = bytes.div_ceil(Self::BYTES_PER_MB);
                Text::format(StratumMessage::NoticeImportTooBig, Self::file(file).uint("mb", megabytes))
            }
            ImportFailure::Unreadable => {
                let args = Self::file(file).message("reason", StratumMessage::ReasonUnreadable);
                Text::format(StratumMessage::NoticeImportFailed, args)
            }
            ImportFailure::NotStructure(_) => {
                let args = Self::file(file).message("reason", StratumMessage::ReasonNotStructure);
                Text::format(StratumMessage::NoticeImportFailed, args)
            }
        }
    }

    fn library_reason(error: &LibraryError) -> StratumMessage {
        match error {
            LibraryError::NotFound => StratumMessage::ReasonNotFound,
            LibraryError::NameEmpty => StratumMessage::ReasonNameEmpty,
            LibraryError::NameTooLong => StratumMessage::ReasonNameTooLong,
            LibraryError::Full { .. } => StratumMessage::ReasonFull,
            LibraryError::BadFileName => StratumMessage::ReasonBadFileName,
        }
    }

    fn marker_reason(error: &MarkerError) -> StratumMessage {
        match error {
            MarkerError::NotFound => StratumMessage::ReasonNotFound,
            MarkerError::NameEmpty => StratumMessage::ReasonNameEmpty,
            MarkerError::NameTooLong => StratumMessage::ReasonNameTooLong,
            MarkerError::Full { .. } => StratumMessage::ReasonMarkersFull,
        }
    }
}
