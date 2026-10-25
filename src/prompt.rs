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
    Dialoguer(#[from] dialoguer::Error),
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
