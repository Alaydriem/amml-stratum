use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

pub struct AtomicFile;

impl AtomicFile {
    // Writes a `.tmp` file beside the target, flushes it, then renames it over the target.
    pub fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let mut tmp_name = OsString::from(path.as_os_str());
        tmp_name.push(".tmp");
        let tmp = Path::new(&tmp_name);
        let written = Self::write_tmp(tmp, bytes).and_then(|()| std::fs::rename(tmp, path));
        if written.is_err() {
            // The target is untouched; a leftover tmp file is only clutter.
            let _ = std::fs::remove_file(tmp);
        }
        written
    }

    fn write_tmp(tmp: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let mut file = std::fs::File::create(tmp)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()
    }
}
