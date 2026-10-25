use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error as ThisError;

use crate::schema::{GildFileSparse, QuestionError};

pub const DISCOVERY_ORDER: [&str; 3] = ["gild.toml", "pyproject.toml", "Cargo.toml"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Gild,
    PyProject,
    Cargo,
}

impl FileKind {
    pub fn for_path(path: &Path) -> Self {
        match path.file_name().and_then(|name| name.to_str()) {
            Some("pyproject.toml") => FileKind::PyProject,
            Some("Cargo.toml") => FileKind::Cargo,
            _ => FileKind::Gild,
        }
    }

    fn table(self) -> &'static [&'static str] {
        match self {
            FileKind::Gild => &[],
            FileKind::PyProject => &["tool", "gild"],
            FileKind::Cargo => &["package", "metadata", "gild"],
        }
    }

    fn describe(self) -> &'static str {
        match self {
            FileKind::Gild => "gild",
            FileKind::PyProject => "[tool.gild]",
            FileKind::Cargo => "[package.metadata.gild]",
        }
    }
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("could not read {}: {}", .0.display(), .1)]
    Read(PathBuf, #[source] io::Error),
    #[error("could not parse {}: {}", .0.display(), .1)]
    Toml(PathBuf, #[source] toml::de::Error),
    #[error("{} has no {} table", .0.display(), .1)]
    NoConfig(PathBuf, &'static str),
    #[error("no gild config in {}: looked for {}", .0.display(), DISCOVERY_ORDER.join(", "))]
    NotFound(PathBuf),
    #[error("{}: {}", .0.display(), .1)]
    Question(PathBuf, #[source] QuestionError),
}

pub fn parse(contents: &str, kind: FileKind, path: &Path) -> Result<GildFileSparse, Error> {
    let document: toml::Table = contents
        .parse()
        .map_err(|source| Error::Toml(path.to_path_buf(), source))?;

    let mut table = toml::Value::Table(document);
    for key in kind.table() {
        table = table
            .get(key)
            .cloned()
            .ok_or_else(|| Error::NoConfig(path.to_path_buf(), kind.describe()))?;
    }

    let parsed = GildFileSparse::deserialize(table)
        .map_err(|source| Error::Toml(path.to_path_buf(), source))?;

    parsed
        .resolved_questions()
        .map_err(|source| Error::Question(path.to_path_buf(), source))?;

    Ok(parsed)
}

pub fn load(path: &Path) -> Result<GildFileSparse, Error> {
    let contents =
        fs::read_to_string(path).map_err(|source| Error::Read(path.to_path_buf(), source))?;

    parse(&contents, FileKind::for_path(path), path)
}

pub fn load_from(config_path: Option<&str>, dir: &Path) -> Result<GildFileSparse, Error> {
    if let Some(path) = config_path {
        return load(Path::new(path));
    }

    for name in DISCOVERY_ORDER {
        let candidate = dir.join(name);
        if !candidate.is_file() {
            continue;
        }

        match load(&candidate) {
            Err(Error::NoConfig(..)) => continue,
            result => return result,
        }
    }

    Err(Error::NotFound(dir.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{BUMP_TYPES, QuestionError, QuestionType, VersionLocation};

    fn fixture(dir: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(dir)
    }
    fn with_questions(questions: &str) -> String {
        format!(
            "artifact_name = \"x\"\n\n\
             [version_source]\n\
             type = \"command\"\n\
             command = \"true\"\n\n\
             {questions}"
        )
    }

    fn parse_questions(questions: &str) -> Result<Vec<crate::schema::Question>, Error> {
        let contents = with_questions(questions);
        let parsed = parse(&contents, FileKind::Gild, Path::new("gild.toml"))?;

        Ok(parsed.resolved_questions().expect("validated during parse"))
    }

    fn ids(questions: &[crate::schema::Question]) -> Vec<&str> {
        questions.iter().map(|q| q.id.as_str()).collect()
    }

    fn titled(questions: &[crate::schema::Question], id: &str) -> String {
        questions
            .iter()
            .find(|q| q.id == id)
            .unwrap_or_else(|| panic!("no question {id}"))
            .title
            .clone()
    }
    #[test]
    fn parses_a_dedicated_gild_file() {
        let parsed = load(&fixture("dedicated").join("gild.toml")).unwrap();

        assert_eq!(parsed.artifact_name, "dedicated");
        assert_eq!(parsed.changefile_directory, ".gild");
        assert_eq!(parsed.version_targets.len(), 2);
        assert_eq!(
            ids(&parsed.resolved_questions().unwrap()),
            ["bump_type", "description", "area", "notes", "breaking"]
        );
    }

    #[test]
    fn parses_pyproject_tool_table() {
        let parsed = load(&fixture("python").join("pyproject.toml")).unwrap();

        assert_eq!(parsed.artifact_name, "widget-py");
        assert_eq!(parsed.changefile_directory, "changes");
        assert_eq!(
            ids(&parsed.resolved_questions().unwrap()),
            ["bump_type", "description", "kind", "summary"]
        );
    }

    #[test]
    fn parses_cargo_metadata_table() {
        let parsed = load(&fixture("rust").join("Cargo.toml")).unwrap();

        assert_eq!(parsed.artifact_name, "widget-rs");
        assert_eq!(parsed.changefile_directory, ".changes");
        assert_eq!(
            ids(&parsed.resolved_questions().unwrap()),
            ["bump_type", "description", "breaking"]
        );
    }

    #[test]
    fn ignores_everything_outside_the_gild_table() {
        let python = load(&fixture("python").join("pyproject.toml")).unwrap();
        let rust = load(&fixture("rust").join("Cargo.toml")).unwrap();

        assert_ne!(python.artifact_name, "widget");
        assert_ne!(rust.artifact_name, "widget");
    }
    #[test]
    fn bump_type_and_description_need_no_mention_in_the_file() {
        let parsed = load(&fixture("polyglot").join("Cargo.toml")).unwrap();

        assert_eq!(
            ids(&parsed.resolved_questions().unwrap()),
            ["bump_type", "description"]
        );
    }

    #[test]
    fn built_ins_carry_their_own_types() {
        let questions = parse_questions("").unwrap();
        assert!(matches!(
            questions[0].question_type,
            QuestionType::Choice { ref options }
                if options == &["major", "minor", "patch"]
        ));
        assert!(matches!(questions[1].question_type, QuestionType::Text));
    }

    #[test]
    fn listing_a_built_in_by_id_changes_its_title() {
        let questions = parse_questions(
            "[[questions]]\nid = \"bump_type\"\ntitle = \"How big is this change?\"\n",
        )
        .unwrap();

        assert_eq!(titled(&questions, "bump_type"), "How big is this change?");
        assert_eq!(ids(&questions), ["bump_type", "description"]);
    }

    #[test]
    fn a_retitled_built_in_keeps_its_type() {
        let questions =
            parse_questions("[[questions]]\nid = \"bump_type\"\ntitle = \"Bump?\"\n").unwrap();

        assert!(matches!(
            questions[0].question_type,
            QuestionType::Choice { ref options } if options.as_slice() == BUMP_TYPES
        ));
    }

    #[test]
    fn both_built_ins_can_be_retitled_at_once() {
        let questions = parse_questions(
            "[[questions]]\nid = \"description\"\ntitle = \"What changed?\"\n\n\
             [[questions]]\nid = \"bump_type\"\ntitle = \"Bump?\"\n",
        )
        .unwrap();

        assert_eq!(ids(&questions), ["bump_type", "description"]);
        assert_eq!(titled(&questions, "description"), "What changed?");
        assert_eq!(titled(&questions, "bump_type"), "Bump?");
    }

    #[test]
    fn built_ins_come_before_the_files_own_questions() {
        let questions = parse_questions(
            "[[questions]]\nid = \"area\"\ntitle = \"Area?\"\ntype = \"text\"\n\n\
             [[questions]]\nid = \"bump_type\"\ntitle = \"Bump?\"\n",
        )
        .unwrap();

        assert_eq!(ids(&questions), ["bump_type", "description", "area"]);
    }

    #[test]
    fn a_built_in_cannot_be_given_a_different_type() {
        let error = parse_questions(
            "[[questions]]\nid = \"bump_type\"\ntitle = \"Bump?\"\ntype = \"text\"\n",
        )
        .unwrap_err();

        assert!(
            matches!(error, Error::Question(_, QuestionError::BuiltinRedefined(ref id)) if id == "bump_type"),
            "got {error:?}"
        );
    }

    #[test]
    fn a_built_in_cannot_be_given_different_options() {
        let error = parse_questions(
            "[[questions]]\nid = \"bump_type\"\ntitle = \"Bump?\"\noptions = [\"big\"]\n",
        )
        .unwrap_err();

        assert!(
            matches!(
                error,
                Error::Question(_, QuestionError::BuiltinRedefined(_))
            ),
            "got {error:?}"
        );
    }
    #[test]
    fn parses_every_question_type() {
        let parsed = load(&fixture("dedicated").join("gild.toml")).unwrap();
        let questions = parsed.resolved_questions().unwrap();

        assert!(matches!(
            questions[2].question_type,
            QuestionType::Choice { ref options } if options == &["cli", "parser", "docs"]
        ));
        assert!(matches!(questions[3].question_type, QuestionType::Text));
        assert!(matches!(questions[4].question_type, QuestionType::Bool));
    }

    #[test]
    fn a_question_of_the_files_own_needs_a_type() {
        let error =
            parse_questions("[[questions]]\nid = \"area\"\ntitle = \"Area?\"\n").unwrap_err();

        assert!(
            matches!(error, Error::Question(_, QuestionError::MissingType(ref id)) if id == "area"),
            "got {error:?}"
        );
    }

    #[test]
    fn a_choice_needs_options() {
        let error =
            parse_questions("[[questions]]\nid = \"a\"\ntitle = \"A?\"\ntype = \"choice\"\n")
                .unwrap_err();

        assert!(
            matches!(
                error,
                Error::Question(_, QuestionError::ChoiceWithoutOptions(_))
            ),
            "got {error:?}"
        );
    }

    #[test]
    fn an_empty_options_list_is_not_a_choice() {
        let error = parse_questions(
            "[[questions]]\nid = \"a\"\ntitle = \"A?\"\ntype = \"choice\"\noptions = []\n",
        )
        .unwrap_err();

        assert!(
            matches!(
                error,
                Error::Question(_, QuestionError::ChoiceWithoutOptions(_))
            ),
            "got {error:?}"
        );
    }

    #[test]
    fn only_a_choice_takes_options() {
        let error = parse_questions(
            "[[questions]]\nid = \"a\"\ntitle = \"A?\"\ntype = \"text\"\noptions = [\"x\"]\n",
        )
        .unwrap_err();

        assert!(
            matches!(
                error,
                Error::Question(_, QuestionError::OptionsOnNonChoice(_))
            ),
            "got {error:?}"
        );
    }

    #[test]
    fn ids_cannot_repeat() {
        let error = parse_questions(
            "[[questions]]\nid = \"a\"\ntitle = \"A?\"\ntype = \"text\"\n\n\
             [[questions]]\nid = \"a\"\ntitle = \"Again?\"\ntype = \"bool\"\n",
        )
        .unwrap_err();

        assert!(
            matches!(error, Error::Question(_, QuestionError::DuplicateId(ref id)) if id == "a"),
            "got {error:?}"
        );
    }

    #[test]
    fn an_id_cannot_collide_with_a_built_in_flag() {
        for id in ["help", "version", "config_path", "non_interactive"] {
            let error = parse_questions(&format!(
                "[[questions]]\nid = \"{id}\"\ntitle = \"T\"\ntype = \"text\"\n"
            ))
            .unwrap_err();

            assert!(
                matches!(error, Error::Question(_, QuestionError::ReservedId(ref got, _)) if got == id),
                "got {error:?}"
            );
        }
    }

    #[test]
    fn two_ids_cannot_produce_the_same_flag() {
        let error = parse_questions(
            "[[questions]]\nid = \"my_flag\"\ntitle = \"A\"\ntype = \"text\"\n\n\
             [[questions]]\nid = \"my-flag\"\ntitle = \"B\"\ntype = \"text\"\n",
        )
        .unwrap_err();

        assert!(
            matches!(
                error,
                Error::Question(_, QuestionError::FlagCollision(ref first, ref second, ref name))
                    if first == "my_flag" && second == "my-flag" && name == "my-flag"
            ),
            "got {error:?}"
        );
    }

    #[test]
    fn an_unknown_question_type_is_rejected() {
        let contents =
            with_questions("[[questions]]\nid = \"a\"\ntitle = \"A?\"\ntype = \"bogus\"\n");
        let error = parse(&contents, FileKind::Gild, Path::new("gild.toml")).unwrap_err();

        assert!(matches!(error, Error::Toml(..)), "got {error:?}");
        assert!(error.to_string().contains("bogus"));
    }

    #[test]
    fn a_misspelled_question_key_is_rejected() {
        let contents = with_questions(
            "[[questions]]\nid = \"a\"\ntitle = \"A?\"\ntype = \"choice\"\noptoins = [\"x\"]\n",
        );
        let error = parse(&contents, FileKind::Gild, Path::new("gild.toml")).unwrap_err();

        assert!(matches!(error, Error::Toml(..)), "got {error:?}");
    }
    #[test]
    fn parses_both_version_location_forms() {
        let parsed = load(&fixture("dedicated").join("gild.toml")).unwrap();

        assert!(matches!(
            parsed.version_source,
            VersionLocation::Regex { ref path, ref pattern }
                if path == "Cargo.toml" && pattern.contains("version")
        ));
        assert!(matches!(
            parsed.version_targets[0],
            VersionLocation::Command { ref command } if command.starts_with("cargo set-version")
        ));
        assert!(matches!(
            parsed.version_targets[1],
            VersionLocation::Regex { ref path, .. } if path == "README.md"
        ));
    }

    #[test]
    fn version_targets_default_to_empty() {
        let parsed = load(&fixture("polyglot").join("Cargo.toml")).unwrap();

        assert!(parsed.questions.is_empty());
        assert!(parsed.version_targets.is_empty());
    }

    #[test]
    fn queries_default_to_empty() {
        let parsed = load(&fixture("polyglot").join("Cargo.toml")).unwrap();

        assert!(parsed.queries.is_empty());
    }

    #[test]
    fn parses_query_aliases() {
        let parsed = load(&fixture("dedicated").join("gild.toml")).unwrap();

        assert_eq!(
            parsed.queries.keys().collect::<Vec<_>>(),
            ["breaking", "descriptions", "minors"]
        );
        assert_eq!(
            parsed.queries.get("descriptions").map(String::as_str),
            Some("[*].description")
        );
    }

    #[test]
    fn changefile_directory_defaults_to_dot_gild() {
        let parsed = load(&fixture("polyglot").join("Cargo.toml")).unwrap();

        assert_eq!(parsed.changefile_directory, ".gild");
    }

    #[test]
    fn an_explicit_changefile_directory_still_wins() {
        let parsed = load(&fixture("python").join("pyproject.toml")).unwrap();

        assert_eq!(parsed.changefile_directory, "changes");
    }

    #[test]
    fn file_kind_comes_from_the_file_name() {
        assert_eq!(
            FileKind::for_path(Path::new("a/pyproject.toml")),
            FileKind::PyProject
        );
        assert_eq!(
            FileKind::for_path(Path::new("a/Cargo.toml")),
            FileKind::Cargo
        );
        assert_eq!(FileKind::for_path(Path::new("a/gild.toml")), FileKind::Gild);
        assert_eq!(
            FileKind::for_path(Path::new("a/whatever.toml")),
            FileKind::Gild
        );
    }
    #[test]
    fn discovery_prefers_a_dedicated_gild_file() {
        let parsed = load_from(None, &fixture("dedicated")).unwrap();

        assert_eq!(parsed.artifact_name, "dedicated");
    }

    #[test]
    fn discovery_steps_over_a_manifest_without_a_gild_table() {
        let parsed = load_from(None, &fixture("polyglot")).unwrap();

        assert_eq!(parsed.artifact_name, "widget-polyglot");
    }

    #[test]
    fn an_explicit_path_wins_over_discovery() {
        let path = fixture("python").join("pyproject.toml");
        let parsed = load_from(Some(path.to_str().unwrap()), &fixture("dedicated")).unwrap();

        assert_eq!(parsed.artifact_name, "widget-py");
    }
    #[test]
    fn a_manifest_without_a_gild_table_is_not_a_config() {
        let error = load(&fixture("bare").join("Cargo.toml")).unwrap_err();

        assert!(matches!(error, Error::NoConfig(..)), "got {error:?}");
        assert!(error.to_string().contains("[package.metadata.gild]"));
    }

    #[test]
    fn discovery_reports_when_nothing_holds_a_config() {
        let error = load_from(None, &fixture("bare")).unwrap_err();

        assert!(matches!(error, Error::NotFound(_)), "got {error:?}");
    }

    #[test]
    fn malformed_toml_is_reported() {
        let error = load(&fixture("malformed").join("gild.toml")).unwrap_err();

        assert!(matches!(error, Error::Toml(..)), "got {error:?}");
    }

    #[test]
    fn a_missing_file_is_reported() {
        let error = load(&fixture("dedicated").join("nope.toml")).unwrap_err();

        assert!(matches!(error, Error::Read(..)), "got {error:?}");
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let contents = format!("typoed_key = 1\n{}", with_questions(""));
        let error = parse(&contents, FileKind::Gild, Path::new("gild.toml")).unwrap_err();

        assert!(matches!(error, Error::Toml(..)), "got {error:?}");
    }
}
