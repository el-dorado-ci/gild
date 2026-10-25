use std::fs;
use std::path::Path;

use clap::Args;

use crate::commands::{Command, Error};
use crate::forge::{self, Change, Forge};
use crate::git::{self, Commit};
use crate::schema::{BUMP_TYPE_ID, BUMP_TYPES, DESCRIPTION_ID, GildFileSparse};
use crate::utils::{self, Changefile};
use crate::version;

#[derive(Args)]
pub struct Generate {
    #[arg(long, help = "Resolve authors and pull requests through the forge API")]
    online: bool,

    #[arg(
        long,
        help = "Write the new version to every target and delete the changefiles"
    )]
    apply: bool,
}

struct Source {
    forge: Box<dyn Forge>,
    repo: String,
    online: bool,
}

impl Source {
    fn credit(&self, commit: &Commit) -> Option<String> {
        self.forge
            .handle(&commit.author)
            .or_else(|| self.lookup(|forge| forge.lookup_handle(&self.repo, &commit.sha)))
    }

    fn change(&self, commit: &Commit) -> Option<Change> {
        self.forge
            .change(&self.repo, &commit.subject)
            .or_else(|| self.lookup(|forge| forge.lookup_change(&self.repo, &commit.sha)))
    }

    fn lookup<T>(&self, ask: impl FnOnce(&dyn Forge) -> Option<T>) -> Option<T> {
        self.online.then(|| ask(self.forge.as_ref())).flatten()
    }
}

impl Command for Generate {
    fn run(&self, gild_file: &GildFileSparse, _interactive: bool) -> Result<(), Error> {
        let source = self.source(gild_file)?;
        let dir = Path::new(&gild_file.changefile_directory);
        let changefiles = utils::read_changefiles(dir)?;

        if changefiles.is_empty() {
            println!(
                "{}: no changefiles in {}",
                gild_file.artifact_name,
                dir.display()
            );
            return Ok(());
        }

        let release = match version::pending_bump(&changefiles) {
            Some(bump) => Some(version::next(
                &version::current(&gild_file.version_source)?,
                bump,
            )?),
            None => None,
        };

        let section = section(source.as_ref(), &changefiles, release.as_deref());

        if !self.apply {
            print!("{section}");
            return Ok(());
        }

        let Some(release) = release else {
            return Err(Error::NothingToRelease);
        };

        for target in &gild_file.version_targets {
            version::write(target, &release)?;
        }

        let changelog = Path::new(&gild_file.changelog_path);
        prepend(changelog, &section)
            .map_err(|source| Error::Changelog(changelog.to_path_buf(), source))?;

        for changefile in &changefiles {
            fs::remove_file(&changefile.path)?;
        }

        eprintln!(
            "gild: {} is now {release}; wrote {} and removed {} changefile(s)",
            gild_file.artifact_name,
            changelog.display(),
            changefiles.len()
        );

        Ok(())
    }
}

fn section(source: Option<&Source>, changefiles: &[Changefile], release: Option<&str>) -> String {
    let heading = match release {
        Some(release) => match utils::command_output("date", &["+%Y-%m-%d"]) {
            Some(today) => format!("## {release} - {today}"),
            None => format!("## {release}"),
        },
        None => "## Unreleased".to_owned(),
    };

    let mut section = format!("{heading}\n\n");

    for bump in BUMP_TYPES {
        let matching: Vec<&Changefile> = changefiles
            .iter()
            .filter(|changefile| field(changefile, BUMP_TYPE_ID) == Some(bump))
            .collect();

        if matching.is_empty() {
            continue;
        }

        section.push_str(&format!("### {bump}\n\n"));
        for changefile in matching {
            let description = field(changefile, DESCRIPTION_ID).unwrap_or("(no description)");

            section.push_str(&entry(source, changefile, description));
            section.push('\n');
        }
        section.push('\n');
    }

    section
}

fn prepend(changelog: &Path, section: &str) -> std::io::Result<()> {
    let existing = fs::read_to_string(changelog).unwrap_or_else(|_| "# Changelog\n".to_owned());

    let (title, rest) = match existing.strip_prefix("# ") {
        Some(after) => match after.split_once('\n') {
            Some((title, rest)) => (format!("# {title}\n"), rest.trim_start_matches('\n')),
            None => (existing.clone(), ""),
        },
        None => (String::new(), existing.as_str()),
    };

    let separator = if title.is_empty() { "" } else { "\n" };

    fs::write(changelog, format!("{title}{separator}{section}{rest}"))
}

impl Generate {
    fn source(&self, gild_file: &GildFileSparse) -> Result<Option<Source>, Error> {
        let forge = match gild_file.forge.as_deref() {
            Some(name) => {
                Some(forge::by_name(name).ok_or_else(|| Error::UnknownForge(name.to_owned()))?)
            }
            None => git::remote_url().as_deref().and_then(forge::detect),
        };

        let repo = git::remote_url().as_deref().and_then(forge::repo_path);

        Ok(forge.zip(repo).map(|(forge, repo)| Source {
            forge,
            repo,
            online: self.online || gild_file.forge_api,
        }))
    }
}

fn entry(source: Option<&Source>, changefile: &Changefile, description: &str) -> String {
    let Some(commit) = git::added_in(&changefile.path) else {
        return format!("- {description}");
    };

    let credit = match source.and_then(|source| source.credit(&commit)) {
        Some(handle) => format!("@{handle}"),
        None => commit.author.name.clone(),
    };

    match source.and_then(|source| source.change(&commit)) {
        Some(change) => format!(
            "- {credit}: {description} ([{}]({}))",
            change.label, change.url
        ),
        None => format!("- {credit}: {description}"),
    }
}

fn field<'a>(changefile: &'a Changefile, key: &str) -> Option<&'a str> {
    changefile
        .fields
        .get(key)
        .and_then(serde_yaml::Value::as_str)
}
