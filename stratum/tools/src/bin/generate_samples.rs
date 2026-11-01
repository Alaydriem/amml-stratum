use std::io::{Error, ErrorKind};
use std::path::PathBuf;

use stratum_tools::SampleBuilder;

fn main() -> std::io::Result<()> {
    let directory = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "samples".to_string()));
    std::fs::create_dir_all(&directory)?;
    let files = SampleBuilder::files().map_err(|error| Error::new(ErrorKind::InvalidData, error))?;
    for (name, bytes) in files {
        std::fs::write(directory.join(&name), &bytes)?;
        println!("{name}: {} bytes", bytes.len());
    }
    Ok(())
}
