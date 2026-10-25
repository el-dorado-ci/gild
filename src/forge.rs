use crate::git::Author;
use crate::utils::command_output;

pub struct Change {
    pub label: String,
    pub url: String,
}

pub trait Forge {
    fn handle(&self, author: &Author) -> Option<String>;

    fn change(&self, repo: &str, subject: &str) -> Option<Change>;

    fn lookup_handle(&self, repo: &str, sha: &str) -> Option<String>;

    fn lookup_change(&self, repo: &str, sha: &str) -> Option<Change>;
}

pub struct GitHub;

impl Forge for GitHub {
    fn handle(&self, author: &Author) -> Option<String> {
        let local = author.email.strip_suffix("@users.noreply.github.com")?;
        let handle = local.split_once('+').map_or(local, |(_, handle)| handle);

        (!handle.is_empty()).then(|| handle.to_owned())
    }

    fn change(&self, repo: &str, subject: &str) -> Option<Change> {
        pull_request_number(subject).map(|number| pull_request(repo, number))
    }

    fn lookup_handle(&self, repo: &str, sha: &str) -> Option<String> {
        api(&format!("repos/{repo}/commits/{sha}"), ".author.login")
    }

    fn lookup_change(&self, repo: &str, sha: &str) -> Option<Change> {
        let number = api(&format!("repos/{repo}/commits/{sha}/pulls"), ".[0].number")?;

        Some(pull_request(repo, &number))
    }
}

fn pull_request(repo: &str, number: &str) -> Change {
    Change {
        label: format!("#{number}"),
        url: format!("https://github.com/{repo}/pull/{number}"),
    }
}

fn api(endpoint: &str, filter: &str) -> Option<String> {
    command_output("gh", &["api", endpoint, "--jq", filter]).filter(|value| value != "null")
}

fn pull_request_number(subject: &str) -> Option<&str> {
    let number = match subject.strip_prefix("Merge pull request #") {
        Some(rest) => rest.split_whitespace().next()?,
        None => subject.rsplit_once("(#")?.1.strip_suffix(')')?,
    };

    let digits = !number.is_empty() && number.chars().all(|character| character.is_ascii_digit());

    digits.then_some(number)
}

pub const NAMES: [&str; 1] = ["github"];

pub fn by_name(name: &str) -> Option<Box<dyn Forge>> {
    match name {
        "github" => Some(Box::new(GitHub)),
        _ => None,
    }
}

pub fn detect(remote: &str) -> Option<Box<dyn Forge>> {
    let host = host_of(remote)?;

    by_host(&host).or_else(|| by_host(&ssh_hostname(&host)?))
}

fn by_host(host: &str) -> Option<Box<dyn Forge>> {
    match host {
        "github.com" => Some(Box::new(GitHub)),
        _ => None,
    }
}

fn ssh_hostname(host: &str) -> Option<String> {
    command_output("ssh", &["-G", host])?
        .lines()
        .find_map(|line| line.strip_prefix("hostname "))
        .map(str::to_owned)
}

pub fn repo_path(remote: &str) -> Option<String> {
    let without_scheme = remote.split_once("://").map_or(remote, |(_, rest)| rest);
    let without_user = without_scheme
        .split_once('@')
        .map_or(without_scheme, |(_, rest)| rest);

    let (_, path) = without_user.split_once(['/', ':'])?;
    let path = path.strip_suffix(".git").unwrap_or(path).trim_matches('/');

    (!path.is_empty()).then(|| path.to_owned())
}

fn host_of(remote: &str) -> Option<String> {
    let without_scheme = match remote.split_once("://") {
        Some((_, rest)) => rest,
        None => remote,
    };

    let without_user = match without_scheme.split_once('@') {
        Some((_, rest)) => rest,
        None => without_scheme,
    };

    let host = without_user
        .split(['/', ':'])
        .next()
        .filter(|host| !host.is_empty())?;

    Some(host.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn author(email: &str) -> Author {
        Author {
            name: "Someone".to_owned(),
            email: email.to_owned(),
        }
    }

    #[test]
    fn reads_the_host_out_of_every_remote_form() {
        for remote in [
            "https://github.com/owner/repo.git",
            "ssh://git@github.com/owner/repo.git",
            "git@github.com:owner/repo.git",
            "github.com:owner/repo",
        ] {
            assert_eq!(host_of(remote).as_deref(), Some("github.com"), "{remote}");
        }
    }

    #[test]
    fn reads_the_host_out_of_an_ssh_alias() {
        assert_eq!(
            host_of("git@github-kassett.com:el-dorado-ci/gild.git").as_deref(),
            Some("github-kassett.com")
        );
    }

    #[test]
    fn a_noreply_address_carries_the_handle() {
        assert_eq!(
            GitHub.handle(&author("121340503+kassett@users.noreply.github.com")),
            Some("kassett".to_owned())
        );
        assert_eq!(
            GitHub.handle(&author("kassett@users.noreply.github.com")),
            Some("kassett".to_owned())
        );
        assert_eq!(
            GitHub.handle(&author("29139614+renovate[bot]@users.noreply.github.com")),
            Some("renovate[bot]".to_owned())
        );
    }

    #[test]
    fn an_ordinary_address_carries_no_handle() {
        assert_eq!(GitHub.handle(&author("someone@example.com")), None);
    }

    #[test]
    fn reads_the_repo_out_of_every_remote_form() {
        for remote in [
            "https://github.com/owner/repo.git",
            "ssh://git@github.com/owner/repo.git",
            "git@github.com:owner/repo.git",
            "git@github-kassett.com:owner/repo.git",
            "https://github.com/owner/repo",
        ] {
            assert_eq!(repo_path(remote).as_deref(), Some("owner/repo"), "{remote}");
        }
    }

    #[test]
    fn a_squashed_subject_carries_the_pull_request() {
        assert_eq!(
            GitHub
                .change("owner/repo", "Refactoring the CLI (#19)")
                .map(|change| (change.label, change.url)),
            Some((
                "#19".to_owned(),
                "https://github.com/owner/repo/pull/19".to_owned()
            ))
        );
    }

    #[test]
    fn a_merge_subject_carries_the_pull_request() {
        assert_eq!(
            GitHub
                .change("owner/repo", "Merge pull request #5 from owner/branch")
                .map(|change| (change.label, change.url)),
            Some((
                "#5".to_owned(),
                "https://github.com/owner/repo/pull/5".to_owned()
            ))
        );
    }

    #[test]
    fn an_ordinary_subject_carries_no_pull_request() {
        for subject in ["initial commands", "fixes (#notanumber)", "ends with (#)"] {
            assert!(GitHub.change("owner/repo", subject).is_none(), "{subject}");
        }
    }

    #[test]
    fn names_resolve_to_a_forge() {
        assert!(by_name("github").is_some());
        assert!(by_name("bitbucket").is_none());
    }
}
