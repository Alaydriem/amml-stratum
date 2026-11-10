use stratum_core::services::AtomicFile;

#[test]
fn atomic_write_leaves_no_tmp_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.hcl");
    AtomicFile::write(&path, b"first").unwrap();
    AtomicFile::write(&path, b"second").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"second");
    let names: Vec<String> =
        std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["data.hcl"]);
}
