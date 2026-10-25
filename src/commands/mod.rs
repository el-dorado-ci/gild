pub mod add;
pub mod ask;
pub mod generate;
pub mod query;
pub mod staged_version;

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
    #[error(transparent)]
    Version(#[from] crate::version::Error),
    #[error("could not find an unused changefile name in {}", .0.display())]
    NoAvailableName(PathBuf),
    #[error("non-interactive mode needs {}", .0.join(" "))]
    MissingAnswers(Vec<String>),
    #[error("could not write {}: {}", .0.display(), .1)]
    Changelog(PathBuf, #[source] io::Error),
    #[error("nothing to release: no changefile sets `bump_type`")]
    NothingToRelease,
    #[error("unknown forge `{}`; gild knows {}", .0, crate::forge::NAMES.join(", "))]
    UnknownForge(String),
    #[error("Invalid JMESPath query '{0}': {1}")]
    InvalidQuery(String, #[source] Box<jmespath::JmespathError>),
    #[error("JMESPath evaluation failed: {0}")]
    QueryFailed(#[source] Box<jmespath::JmespathError>),
    #[error("no query named `{}` in the gild file; defined queries: {}", .0, defined(.1))]
    UnknownQuery(String, Vec<String>),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn defined(aliases: &[String]) -> String {
    if aliases.is_empty() {
        "none".to_owned()
    } else {
        aliases.join(", ")
    }
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
        Subcommand::Ask(ask) => ask.run(gild_file, interactive),
        Subcommand::StagedVersion(staged) => staged.run(gild_file, interactive),
    }
}
