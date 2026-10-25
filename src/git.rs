use std::path::Path;

use crate::utils::command_output;

pub struct Author {
    pub name: String,
    pub email: String,
}

pub struct Commit {
    pub sha: String,
    pub author: Author,
    pub subject: String,
}

pub fn remote_url() -> Option<String> {
    command_output("git", &["remote", "get-url", "origin"])
}

pub fn added_in(path: &Path) -> Option<Commit> {
    let line = command_output(
        "git",
        &[
            "log",
            "--diff-filter=A",
            "--follow",
            "--format=%H\t%an\t%ae\t%s",
            "-1",
            "--",
            path.to_str()?,
        ],
    )?;

    let mut parts = line.splitn(4, '\t');
    let sha = parts.next()?;
    let name = parts.next()?;
    let email = parts.next()?;
    let subject = parts.next()?;

    Some(Commit {
        sha: sha.to_owned(),
        author: Author {
            name: name.to_owned(),
            email: email.to_owned(),
        },
        subject: subject.to_owned(),
    })
}
