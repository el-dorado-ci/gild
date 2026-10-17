use std::fmt;

use serde::Deserialize;

// Always asked, whether or not the gild file mentions them. A gild file may
// list them by id to change their title, and nothing else.
pub const BUMP_TYPE_ID: &str = "bump_type";
pub const DESCRIPTION_ID: &str = "description";

// A closed set: a gild file can retitle the bump question but cannot add to,
// remove from or reorder its answers.
pub const BUMP_TYPES: [&str; 4] = ["major", "minor", "patch", "prerelease"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionType {
    Choice { options: Vec<String> },
    Text,
    Bool,
}

// A question gild will ask.
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

// A question as written in the gild file. `tag` is optional so a built-in can
// be retitled with nothing but an id and a title; resolving turns the pair
// into a QuestionType, or explains why it cannot.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionError {
    MissingType(String),
    ChoiceWithoutOptions(String),
    OptionsOnNonChoice(String),
    BuiltinRedefined(String),
    DuplicateId(String),
}

impl fmt::Display for QuestionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuestionError::MissingType(id) => {
                write!(f, "question `{id}` needs a type of choice, text or bool")
            }
            QuestionError::ChoiceWithoutOptions(id) => {
                write!(f, "choice question `{id}` needs a non-empty options list")
            }
            QuestionError::OptionsOnNonChoice(id) => {
                write!(f, "question `{id}` is not a choice, so it takes no options")
            }
            QuestionError::BuiltinRedefined(id) => write!(
                f,
                "`{id}` is built in; a gild file may only change its title"
            ),
            QuestionError::DuplicateId(id) => write!(f, "question `{id}` is defined twice"),
        }
    }
}

impl std::error::Error for QuestionError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GildFileSparse {
    pub artifact_name: String,
    #[serde(default = "default_changefile_directory")]
    pub changefile_directory: String,
    #[serde(default)]
    pub questions: Vec<QuestionSparse>,
    pub version_source: VersionLocation,
    #[serde(default)]
    pub version_targets: Vec<VersionLocation>,
}

impl GildFileSparse {
    // The built-ins come first, then anything the gild file adds, in the order
    // it lists them. Retitling a built-in does not move it.
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

        Ok(resolved)
    }
}

fn default_changefile_directory() -> String {
    ".gild".to_owned()
}

// A version is either read out of a file with a regex, or produced by running a
// command. The same two forms describe where a version gets written back to.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VersionLocation {
    // `pattern` must hold exactly one capture group: the version itself.
    Regex { path: String, pattern: String },
    Command { command: String },
}
