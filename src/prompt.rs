use std::io::{self, BufRead, Write};

use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input, Select};
use thiserror::Error as ThisError;

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

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Dialoguer(#[from] dialoguer::Error),
    #[error("stdin closed before `{0}` was answered")]
    Eof(String),
    #[error("`{input}` is not a valid answer for `{id}`")]
    InvalidChoice { id: String, input: String },
    #[error("`{input}` is not yes or no for `{id}`")]
    InvalidBool { id: String, input: String },
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

    fn prompt(&mut self, question: &Question, hint: &str) -> Result<String, Error> {
        eprintln!("{}{hint}", question.title);
        io::stderr().flush()?;

        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            return Err(Error::Eof(question.id.clone()));
        }

        Ok(line.trim().to_owned())
    }
}

impl<R: BufRead> Prompter for NonInteractive<R> {
    fn ask(&mut self, question: &Question) -> Result<Answer, Error> {
        let id = || question.id.clone();

        match &question.question_type {
            QuestionType::Choice { options } => {
                let input = self.prompt(question, &format!(" [{}]", options.join(", ")))?;

                options
                    .iter()
                    .find(|option| option.eq_ignore_ascii_case(&input))
                    .cloned()
                    .map(Answer::Choice)
                    .ok_or(Error::InvalidChoice { id: id(), input })
            }
            QuestionType::Text => Ok(Answer::Text(self.prompt(question, "")?)),
            QuestionType::Bool => {
                let input = self.prompt(question, " [y/n]")?;

                match input.to_ascii_lowercase().as_str() {
                    "y" | "yes" | "true" => Ok(Answer::Bool(true)),
                    "n" | "no" | "false" => Ok(Answer::Bool(false)),
                    _ => Err(Error::InvalidBool { id: id(), input }),
                }
            }
        }
    }
}
