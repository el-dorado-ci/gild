pub mod add;
pub mod generate;
pub mod query;

use std::io;
use std::path::PathBuf;

use thiserror::Error as ThisError;

use crate::env::Command as Subcommand;
use crate::prompt;
use crate::schema::{GildFileSparse, QuestionError};
use crate::utils;

pub trait Command {
    fn run(&self, gild_file: &GildFileSparse, interactive: bool) -> Result<(), Error>;
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Question(#[from] QuestionError),
    #[error(transparent)]
    Prompt(#[from] prompt::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Changefile(#[from] utils::Error),
    #[error("could not find an unused changefile name in {}", .0.display())]
    NoAvailableName(PathBuf),
    #[error("Invalid JMESPath query '{0}': {1}")]
    InvalidQuery(String, #[source] Box<jmespath::JmespathError>),
    #[error("JMESPath evaluation failed: {0}")]
    QueryFailed(#[source] Box<jmespath::JmespathError>),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub fn dispatch(
    command: Subcommand,
    gild_file: &GildFileSparse,
    interactive: bool,
) -> Result<(), Error> {
    match command {
        Subcommand::Add(add) => add.run(gild_file, interactive),
        Subcommand::Generate(generate) => generate.run(gild_file, interactive),
        Subcommand::Query(query) => query.run(gild_file, interactive),
    }
}
