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
    pub path: PathBuf,
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

            Ok(Changefile { path, fields })
        })
        .collect()
}

pub fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();

    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}
