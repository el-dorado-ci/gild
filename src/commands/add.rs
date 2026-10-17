use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rand::seq::SliceRandom;

use crate::prompt::{self, Prompter};
use crate::schema::{GildFileSparse, QuestionError};
use crate::words;

#[derive(Debug)]
pub enum Error {
    Question(QuestionError),
    Prompt(prompt::Error),
    Io(io::Error),
    Yaml(serde_yaml::Error),
    NoAvailableName(PathBuf),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Question(source) => write!(f, "{source}"),
            Error::Prompt(source) => write!(f, "{source}"),
            Error::Io(source) => write!(f, "{source}"),
            Error::Yaml(source) => write!(f, "{source}"),
            Error::NoAvailableName(dir) => {
                write!(
                    f,
                    "could not find an unused changefile name in {}",
                    dir.display()
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Question(source) => Some(source),
            Error::Prompt(source) => Some(source),
            Error::Io(source) => Some(source),
            Error::Yaml(source) => Some(source),
            Error::NoAvailableName(_) => None,
        }
    }
}

impl From<QuestionError> for Error {
    fn from(source: QuestionError) -> Self {
        Error::Question(source)
    }
}

impl From<prompt::Error> for Error {
    fn from(source: prompt::Error) -> Self {
        Error::Prompt(source)
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Error::Io(source)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(source: serde_yaml::Error) -> Self {
        Error::Yaml(source)
    }
}

pub fn run(gild_file: &GildFileSparse, interactive: bool) -> Result<(), Error> {
    let questions = gild_file.resolved_questions()?;

    let stdin = io::stdin();
    let mut interactive_prompter = prompt::Interactive;
    let mut non_interactive_prompter = prompt::NonInteractive::new(stdin.lock());

    let prompter: &mut dyn Prompter = if interactive {
        &mut interactive_prompter
    } else {
        &mut non_interactive_prompter
    };

    let mut answers = serde_yaml::Mapping::new();
    for question in &questions {
        let answer = prompter.ask(question)?;
        answers.insert(
            serde_yaml::Value::String(question.id.clone()),
            answer.into_yaml(),
        );
    }

    let dir = Path::new(&gild_file.changefile_directory);
    fs::create_dir_all(dir)?;

    let path = unique_changefile_path(dir)?;
    fs::write(&path, serde_yaml::to_string(&answers)?)?;

    println!("wrote {}", path.display());

    Ok(())
}

fn unique_changefile_path(dir: &Path) -> Result<PathBuf, Error> {
    let mut rng = rand::thread_rng();

    for _ in 0..50 {
        let adjective = words::ADJECTIVES.choose(&mut rng).expect("non-empty");
        let noun = words::NOUNS.choose(&mut rng).expect("non-empty");
        let verb = words::VERBS.choose(&mut rng).expect("non-empty");

        let candidate = dir.join(format!("{adjective}-{noun}-{verb}.yaml"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(Error::NoAvailableName(dir.to_path_buf()))
}
