use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
    CoUninitialize,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FileOpenDialog, IFileOpenDialog, IShellItemArray,
    SIGDN_FILESYSPATH,
};
use windows::core::{PCWSTR, w};

use crate::traits::DialogRunner;
use crate::types::PickResult;

// The Windows file-open dialog, filtered to .mcstructure, with more than one file allowed and no owner window. It
// blocks, so it runs on the picker's own thread
pub struct FileDialog;

impl FileDialog {
    // fixed text, outside amml-i18n, as the LeviLamina build had it
    pub const TITLE: PCWSTR = w!("Import structures");
    pub const FILTER_LABEL: PCWSTR = w!("Minecraft structure");
    pub const FILTER_SPEC: PCWSTR = w!("*.mcstructure");

    pub fn new() -> Self {
        Self
    }

    // the chosen items' paths, with the status of the first failing call
    #[allow(unsafe_code)]
    fn show() -> PickResult {
        let mut result = PickResult { status: 0, paths: Vec::new(), dropped: 0, invalid: 0 };
        let items = match Self::run() {
            Ok(items) => items,
            Err(error) => {
                result.status = error.code().0;
                return result;
            }
        };
        // SAFETY: `items` is the live result array of the dialog
        let count = match unsafe { items.GetCount() } {
            Ok(count) => count,
            Err(error) => {
                result.status = error.code().0;
                return result;
            }
        };
        result.paths.reserve(count as usize);
        for index in 0..count {
            // SAFETY: `index` is below the array's count
            let path = unsafe { items.GetItemAt(index) }.and_then(|item| {
                // SAFETY: `item` is a live shell item; the dialog was asked for file system items only
                unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }
            });
            let Ok(path) = path else {
                result.dropped += 1;
                continue;
            };
            // SAFETY: GetDisplayName returns a NUL-terminated wide string that the caller owns
            let text = String::from_utf16(unsafe { path.as_wide() });
            // SAFETY: the string came from the COM task allocator and is not read again
            unsafe { CoTaskMemFree(Some(path.0.cast_const().cast())) };
            match text {
                Ok(text) => result.paths.push(text),
                Err(_) => result.invalid += 1,
            }
        }
        result
    }

    // the dialog with its options, filter and title, shown modal; its results once it closes with OK
    #[allow(unsafe_code)]
    fn run() -> windows::core::Result<IShellItemArray> {
        // SAFETY: COM is initialised on this thread (pick) and the class id names the file-open dialog
        let dialog: IFileOpenDialog = unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL) }?;
        let filter = [COMDLG_FILTERSPEC { pszName: Self::FILTER_LABEL, pszSpec: Self::FILTER_SPEC }];
        // SAFETY: each call is made on the live dialog with arguments that outlive the call
        unsafe {
            let options = dialog.GetOptions()?;
            dialog.SetOptions(options | FOS_ALLOWMULTISELECT | FOS_FILEMUSTEXIST | FOS_FORCEFILESYSTEM)?;
            dialog.SetFileTypes(&filter)?;
            dialog.SetTitle(Self::TITLE)?;
            dialog.Show(None)?;
            dialog.GetResults()
        }
    }
}

impl DialogRunner for FileDialog {
    // initialises COM for this thread around the dialog; a failed initialisation is the answer's status
    #[allow(unsafe_code)]
    fn pick(&self) -> PickResult {
        // SAFETY: called on the picker's own thread, which has not initialised COM
        let init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        if init.is_err() {
            return PickResult { status: init.0, paths: Vec::new(), dropped: 0, invalid: 0 };
        }
        let result = Self::show();
        // SAFETY: pairs the successful CoInitializeEx above; every COM object of the dialog is released by now
        unsafe { CoUninitialize() };
        result
    }
}

impl Default for FileDialog {
    fn default() -> Self {
        Self::new()
    }
}
