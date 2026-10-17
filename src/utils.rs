use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("could not read {}: {}", .0.display(), .1)]
    Read(PathBuf, #[source] io::Error),
    #[error("could not parse {}: {}", .0.display(), .1)]
    Parse(PathBuf, #[source] serde_yaml::Error),
}

pub struct Changefile {
    pub name: String,
    pub fields: serde_yaml::Mapping,
}

pub fn list_changefiles(dir: &Path) -> io::Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .collect();
    paths.sort();

    Ok(paths)
}

pub fn read_changefiles(dir: &Path) -> Result<Vec<Changefile>, Error> {
    list_changefiles(dir)
        .map_err(|source| Error::Read(dir.to_path_buf(), source))?
        .into_iter()
        .map(|path| {
            let contents =
                fs::read_to_string(&path).map_err(|source| Error::Read(path.clone(), source))?;
            let fields = serde_yaml::from_str(&contents)
                .map_err(|source| Error::Parse(path.clone(), source))?;
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("?")
                .to_owned();

            Ok(Changefile { name, fields })
        })
        .collect()
}
