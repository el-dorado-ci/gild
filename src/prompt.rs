use std::fmt;
use std::io::{self, BufRead, Write};

use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input, Select};

use crate::schema::{Question, QuestionType};

#[derive(Debug, Clone)]
pub enum Answer {
    Text(String),
    Bool(bool),
    Choice(String),
}

impl Answer {
    pub fn into_yaml(self) -> serde_yaml::Value {
        match self {
            Answer::Text(value) | Answer::Choice(value) => serde_yaml::Value::String(value),
            Answer::Bool(value) => serde_yaml::Value::Bool(value),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Dialoguer(dialoguer::Error),
    Eof(String),
    InvalidChoice { id: String, input: String },
    InvalidBool { id: String, input: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(source) => write!(f, "{source}"),
            Error::Dialoguer(source) => write!(f, "{source}"),
            Error::Eof(id) => write!(f, "stdin closed before `{id}` was answered"),
            Error::InvalidChoice { id, input } => {
                write!(f, "`{input}` is not a valid answer for `{id}`")
            }
            Error::InvalidBool { id, input } => {
                write!(f, "`{input}` is not yes or no for `{id}`")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(source) => Some(source),
            Error::Dialoguer(source) => Some(source),
            Error::Eof(_) | Error::InvalidChoice { .. } | Error::InvalidBool { .. } => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Error::Io(source)
    }
}

impl From<dialoguer::Error> for Error {
    fn from(source: dialoguer::Error) -> Self {
        Error::Dialoguer(source)
    }
}

pub trait Prompter {
    fn ask(&mut self, question: &Question) -> Result<Answer, Error>;
}

pub struct Interactive;

impl Prompter for Interactive {
    fn ask(&mut self, question: &Question) -> Result<Answer, Error> {
        let theme = ColorfulTheme::default();

        match &question.question_type {
            QuestionType::Choice { options } => {
                let index = Select::with_theme(&theme)
                    .with_prompt(&question.title)
                    .items(options)
                    .default(0)
                    .interact()?;

                Ok(Answer::Choice(options[index].clone()))
            }
            QuestionType::Text => {
                let value = Input::<String>::with_theme(&theme)
                    .with_prompt(&question.title)
                    .allow_empty(true)
                    .interact_text()?;

                Ok(Answer::Text(value))
            }
            QuestionType::Bool => {
                let value = Confirm::with_theme(&theme)
                    .with_prompt(&question.title)
                    .interact()?;

                Ok(Answer::Bool(value))
            }
        }
    }
}

pub struct NonInteractive<R> {
    reader: R,
}

impl<R: BufRead> NonInteractive<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    fn read_line(&mut self, question: &Question) -> Result<String, Error> {
        io::stderr().flush()?;

        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(Error::Eof(question.id.clone()));
        }

        Ok(line.trim().to_owned())
    }
}

impl<R: BufRead> Prompter for NonInteractive<R> {
    fn ask(&mut self, question: &Question) -> Result<Answer, Error> {
        match &question.question_type {
            QuestionType::Choice { options } => {
                eprintln!("{} [{}]", question.title, options.join(", "));
                let input = self.read_line(question)?;

                options
                    .iter()
                    .find(|option| option.eq_ignore_ascii_case(&input))
                    .cloned()
                    .map(Answer::Choice)
                    .ok_or_else(|| Error::InvalidChoice {
                        id: question.id.clone(),
                        input,
                    })
            }
            QuestionType::Text => {
                eprintln!("{}", question.title);
                let input = self.read_line(question)?;

                Ok(Answer::Text(input))
            }
            QuestionType::Bool => {
                eprintln!("{} [y/n]", question.title);
                let input = self.read_line(question)?;

                match input.to_ascii_lowercase().as_str() {
                    "y" | "yes" | "true" => Ok(Answer::Bool(true)),
                    "n" | "no" | "false" => Ok(Answer::Bool(false)),
                    _ => Err(Error::InvalidBool {
                        id: question.id.clone(),
                        input,
                    }),
                }
            }
        }
    }
}
