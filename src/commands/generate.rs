use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::schema::{BUMP_TYPE_ID, BUMP_TYPES, DESCRIPTION_ID, GildFileSparse};

#[derive(Debug)]
pub enum Error {
    Io(PathBuf, io::Error),
    Yaml(PathBuf, serde_yaml::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(path, source) => write!(f, "could not read {}: {source}", path.display()),
            Error::Yaml(path, source) => {
                write!(f, "could not parse {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(_, source) => Some(source),
            Error::Yaml(_, source) => Some(source),
        }
    }
}

struct Changefile {
    name: String,
    fields: serde_yaml::Mapping,
}

pub fn run(gild_file: &GildFileSparse) -> Result<(), Error> {
    let dir = PathBuf::from(&gild_file.changefile_directory);
    let changefiles = load_changefiles(&dir)?;

    if changefiles.is_empty() {
        println!(
            "{}: no changefiles in {}",
            gild_file.artifact_name,
            dir.display()
        );
        return Ok(());
    }

    println!("# {} changelog preview\n", gild_file.artifact_name);

    let mut wrote_a_section = false;
    for bump in BUMP_TYPES {
        let matching: Vec<&Changefile> = changefiles
            .iter()
            .filter(|changefile| bump_type_of(changefile) == Some(bump))
            .collect();

        if matching.is_empty() {
            continue;
        }

        wrote_a_section = true;
        println!("## {bump}\n");
        for changefile in matching {
            let description = changefile
                .fields
                .get(DESCRIPTION_ID)
                .and_then(serde_yaml::Value::as_str)
                .unwrap_or("(no description)");

            println!("- {description}  _{}_", changefile.name);
        }
        println!();
    }

    if !wrote_a_section {
        println!(
            "{} changefile(s) found, but none set `{BUMP_TYPE_ID}`",
            changefiles.len()
        );
    }

    Ok(())
}

fn bump_type_of(changefile: &Changefile) -> Option<&str> {
    changefile
        .fields
        .get(BUMP_TYPE_ID)
        .and_then(serde_yaml::Value::as_str)
}

fn load_changefiles(dir: &std::path::Path) -> Result<Vec<Changefile>, Error> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|source| Error::Io(dir.to_path_buf(), source))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .collect();
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let contents =
                fs::read_to_string(&path).map_err(|source| Error::Io(path.clone(), source))?;
            let fields = serde_yaml::from_str(&contents)
                .map_err(|source| Error::Yaml(path.clone(), source))?;
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("?")
                .to_owned();

            Ok(Changefile { name, fields })
        })
        .collect()
}
