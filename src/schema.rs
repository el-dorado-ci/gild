use std::collections::BTreeMap;

use serde::Deserialize;
use thiserror::Error;

pub const BUMP_TYPE_ID: &str = "bump_type";
pub const DESCRIPTION_ID: &str = "description";

pub const BUMP_TYPES: [&str; 3] = ["major", "minor", "patch"];

const RESERVED_FLAGS: [&str; 4] = ["help", "version", "config-path", "non-interactive"];

pub fn flag(id: &str) -> String {
    id.replace('_', "-")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionType {
    Choice { options: Vec<String> },
    Text,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub title: String,
    pub question_type: QuestionType,
}

pub fn builtin_questions() -> Vec<Question> {
    vec![
        Question {
            id: BUMP_TYPE_ID.to_owned(),
            title: "How should this change the version?".to_owned(),
            question_type: QuestionType::Choice {
                options: BUMP_TYPES.iter().map(|bump| (*bump).to_owned()).collect(),
            },
        },
        Question {
            id: DESCRIPTION_ID.to_owned(),
            title: "Describe the change".to_owned(),
            question_type: QuestionType::Text,
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionTag {
    Choice,
    Text,
    Bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionSparse {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub tag: Option<QuestionTag>,
    pub options: Option<Vec<String>>,
}

impl QuestionSparse {
    fn resolve(&self) -> Result<Question, QuestionError> {
        let question_type = match (self.tag, self.options.as_deref()) {
            (None, _) => return Err(QuestionError::MissingType(self.id.clone())),
            (Some(QuestionTag::Choice), None | Some([])) => {
                return Err(QuestionError::ChoiceWithoutOptions(self.id.clone()));
            }
            (Some(QuestionTag::Choice), Some(options)) => QuestionType::Choice {
                options: options.to_vec(),
            },
            (Some(QuestionTag::Text | QuestionTag::Bool), Some(_)) => {
                return Err(QuestionError::OptionsOnNonChoice(self.id.clone()));
            }
            (Some(QuestionTag::Text), None) => QuestionType::Text,
            (Some(QuestionTag::Bool), None) => QuestionType::Bool,
        };

        Ok(Question {
            id: self.id.clone(),
            title: self.title.clone(),
            question_type,
        })
    }

    fn only_retitles(&self) -> bool {
        self.tag.is_none() && self.options.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum QuestionError {
    #[error("question `{0}` needs a type of choice, text or bool")]
    MissingType(String),
    #[error("choice question `{0}` needs a non-empty options list")]
    ChoiceWithoutOptions(String),
    #[error("question `{0}` is not a choice, so it takes no options")]
    OptionsOnNonChoice(String),
    #[error("`{0}` is built in; a gild file may only change its title")]
    BuiltinRedefined(String),
    #[error("question `{0}` is defined twice")]
    DuplicateId(String),
    #[error("question `{0}` would collide with gild's own `--{1}` flag")]
    ReservedId(String, String),
    #[error("questions `{0}` and `{1}` would both become `--{2}`")]
    FlagCollision(String, String, String),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GildFileSparse {
    pub artifact_name: String,
    #[serde(default = "default_changefile_directory")]
    pub changefile_directory: String,
    #[serde(default)]
    pub questions: Vec<QuestionSparse>,
    #[serde(default)]
    pub queries: BTreeMap<String, String>,
    #[allow(dead_code)]
    pub version_source: VersionLocation,
    #[serde(default)]
    #[allow(dead_code)]
    pub version_targets: Vec<VersionLocation>,
}

impl GildFileSparse {
    pub fn resolved_questions(&self) -> Result<Vec<Question>, QuestionError> {
        let mut resolved = builtin_questions();
        let mut seen: Vec<&str> = Vec::new();

        for sparse in &self.questions {
            if seen.contains(&sparse.id.as_str()) {
                return Err(QuestionError::DuplicateId(sparse.id.clone()));
            }
            seen.push(&sparse.id);

            match resolved.iter_mut().find(|builtin| builtin.id == sparse.id) {
                Some(builtin) if sparse.only_retitles() => builtin.title.clone_from(&sparse.title),
                Some(_) => return Err(QuestionError::BuiltinRedefined(sparse.id.clone())),
                None => resolved.push(sparse.resolve()?),
            }
        }

        validate_flags(&resolved)?;

        Ok(resolved)
    }
}

fn validate_flags(questions: &[Question]) -> Result<(), QuestionError> {
    let mut taken: Vec<(String, &str)> = Vec::new();

    for question in questions {
        let name = flag(&question.id);

        if RESERVED_FLAGS.contains(&name.as_str()) {
            return Err(QuestionError::ReservedId(question.id.clone(), name));
        }

        if let Some((_, owner)) = taken.iter().find(|(taken, _)| *taken == name) {
            return Err(QuestionError::FlagCollision(
                (*owner).to_owned(),
                question.id.clone(),
                name,
            ));
        }

        taken.push((name, &question.id));
    }

    Ok(())
}

fn default_changefile_directory() -> String {
    ".gild".to_owned()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum VersionLocation {
    Regex { path: String, pattern: String },
    Command { command: String },
}
