use std::fs;
use std::path::PathBuf;

use regex::RegexBuilder;
use thiserror::Error as ThisError;

use crate::schema::{BUMP_TYPE_ID, VersionLocation};
use crate::utils::{Changefile, command_output};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bump {
    Patch,
    Minor,
    Major,
}

impl Bump {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "major" => Some(Bump::Major),
            "minor" => Some(Bump::Minor),
            "patch" => Some(Bump::Patch),
            _ => None,
        }
    }
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("could not read {}: {}", .0.display(), .1)]
    Read(PathBuf, #[source] std::io::Error),
    #[error("could not write {}: {}", .0.display(), .1)]
    Write(PathBuf, #[source] std::io::Error),
    #[error("`{0}` is not a valid regex: {1}")]
    Pattern(String, #[source] regex::Error),
    #[error("`{}` needs exactly one capture group, found {}", .0, .1)]
    Captures(String, usize),
    #[error("`{}` found no version in {}", .0, .1.display())]
    NoMatch(String, PathBuf),
    #[error("`{0}` produced no version")]
    NoOutput(String),
    #[error("`{0}` is not a semantic version: {1}")]
    NotSemver(String, #[source] semver::Error),
}

pub fn pending_bump(changefiles: &[Changefile]) -> Option<Bump> {
    changefiles
        .iter()
        .filter_map(|changefile| {
            changefile
                .fields
                .get(BUMP_TYPE_ID)
                .and_then(serde_yaml::Value::as_str)
                .and_then(Bump::parse)
        })
        .max()
}

pub fn current(source: &VersionLocation) -> Result<String, Error> {
    match source {
        VersionLocation::Regex { path, pattern } => {
            let path = PathBuf::from(path);
            let contents =
                fs::read_to_string(&path).map_err(|source| Error::Read(path.clone(), source))?;
            let regex = compile(pattern)?;

            regex
                .captures(&contents)
                .and_then(|captures| captures.get(1))
                .map(|found| found.as_str().to_owned())
                .ok_or_else(|| Error::NoMatch(pattern.clone(), path))
        }
        VersionLocation::Command { command } => {
            command_output("sh", &["-c", command]).ok_or_else(|| Error::NoOutput(command.clone()))
        }
    }
}

pub fn next(current: &str, bump: Bump) -> Result<String, Error> {
    let mut version = semver::Version::parse(current)
        .map_err(|source| Error::NotSemver(current.to_owned(), source))?;

    version.pre = semver::Prerelease::EMPTY;
    version.build = semver::BuildMetadata::EMPTY;

    match bump {
        Bump::Major => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
        }
        Bump::Minor => {
            version.minor += 1;
            version.patch = 0;
        }
        Bump::Patch => version.patch += 1,
    }

    Ok(version.to_string())
}

pub fn write(target: &VersionLocation, version: &str) -> Result<(), Error> {
    match target {
        VersionLocation::Regex { path, pattern } => {
            let path = PathBuf::from(path);
            let contents =
                fs::read_to_string(&path).map_err(|source| Error::Read(path.clone(), source))?;
            let regex = compile(pattern)?;

            let found = regex
                .captures(&contents)
                .and_then(|captures| captures.get(1))
                .ok_or_else(|| Error::NoMatch(pattern.clone(), path.clone()))?;

            let mut updated = String::with_capacity(contents.len());
            updated.push_str(&contents[..found.start()]);
            updated.push_str(version);
            updated.push_str(&contents[found.end()..]);

            fs::write(&path, updated).map_err(|source| Error::Write(path, source))
        }
        VersionLocation::Command { command } => {
            let expanded = command.replace("$GILD_VERSION", version);

            command_output("sh", &["-c", &expanded])
                .map(|_| ())
                .ok_or_else(|| Error::NoOutput(command.clone()))
        }
    }
}

fn compile(pattern: &str) -> Result<regex::Regex, Error> {
    let regex = RegexBuilder::new(pattern)
        .multi_line(true)
        .build()
        .map_err(|source| Error::Pattern(pattern.to_owned(), source))?;

    match regex.captures_len() {
        2 => Ok(regex),
        groups => Err(Error::Captures(pattern.to_owned(), groups - 1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bumps_follow_semver() {
        assert_eq!(next("0.1.0", Bump::Patch).unwrap(), "0.1.1");
        assert_eq!(next("0.1.0", Bump::Minor).unwrap(), "0.2.0");
        assert_eq!(next("0.1.0", Bump::Major).unwrap(), "1.0.0");
        assert_eq!(next("1.2.3", Bump::Minor).unwrap(), "1.3.0");
    }

    #[test]
    fn a_bump_clears_any_prerelease() {
        assert_eq!(next("1.0.0-rc.1", Bump::Patch).unwrap(), "1.0.1");
    }

    #[test]
    fn a_non_semver_version_is_rejected() {
        assert!(matches!(
            next("banana", Bump::Patch),
            Err(Error::NotSemver(..))
        ));
    }

    #[test]
    fn the_largest_pending_bump_wins() {
        assert_eq!(pending_bump(&[]), None);
        assert_eq!(
            pending_bump(&[
                changefile("patch"),
                changefile("major"),
                changefile("minor")
            ]),
            Some(Bump::Major)
        );
        assert_eq!(
            pending_bump(&[changefile("patch"), changefile("minor")]),
            Some(Bump::Minor)
        );
    }

    #[test]
    fn a_pattern_needs_exactly_one_capture_group() {
        assert!(compile("^version = (.+)$").is_ok());
        assert!(matches!(
            compile("^version = .+$"),
            Err(Error::Captures(_, 0))
        ));
        assert!(matches!(compile("^(a)(b)$"), Err(Error::Captures(_, 2))));
        assert!(matches!(compile("^(unclosed$"), Err(Error::Pattern(..))));
    }

    fn changefile(bump: &str) -> Changefile {
        let mut fields = serde_yaml::Mapping::new();
        fields.insert(
            serde_yaml::Value::String(BUMP_TYPE_ID.to_owned()),
            serde_yaml::Value::String(bump.to_owned()),
        );

        Changefile {
            path: PathBuf::from("x.yaml"),
            fields,
        }
    }
}
