pub mod add;
pub mod generate;

use std::fmt;

use crate::env::Command;
use crate::schema::GildFileSparse;

#[derive(Debug)]
pub enum Error {
    Add(add::Error),
    Generate(generate::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Add(source) => write!(f, "{source}"),
            Error::Generate(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Add(source) => Some(source),
            Error::Generate(source) => Some(source),
        }
    }
}

impl From<add::Error> for Error {
    fn from(source: add::Error) -> Self {
        Error::Add(source)
    }
}

impl From<generate::Error> for Error {
    fn from(source: generate::Error) -> Self {
        Error::Generate(source)
    }
}

pub fn dispatch(
    command: Command,
    gild_file: &GildFileSparse,
    interactive: bool,
) -> Result<(), Error> {
    match command {
        Command::Add => add::run(gild_file, interactive)?,
        Command::Generate => generate::run(gild_file)?,
    }

    Ok(())
}
