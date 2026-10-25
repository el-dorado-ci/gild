

## Description

A CHANGELOG creation tool that brings the sentiment behind pull-request templates into a more structured format.

### In Depth

gild could be called YACT -- Yet Another Changelog Tool. When I entered the software development space,
certainly a CHANGELOG never excited me. But after years of working on repositories contributed to by developers
of vastly different experience, attempting to build workflows that ensure that the right pieces get the right 
reviewers or the right tests, I am resigned to believing that at some point, someone just needs to make a judgment call. 
This is what pull requests templates attempt to do, and they're fine, but they're basically unparseable.

This project is inspired by changesets (https://github.com/changesets/changesets), but provides a few added features
that I think make it really handy.

## Attribution

`gild generate` credits each change to whoever added its changefile, and links the pull request it arrived in.
Both are worked out from git history alone: the author comes from the commit that added the file, and the pull
request from that commit's subject (`Add the thing (#19)`, or `Merge pull request #19 from ...`).

That only goes so far. A handle can be recovered from a commit only when its author used their
`@users.noreply.github.com` address, and a pull request only when its number survived into the commit subject.
Anything that fails either test falls back to the plain git name, with no link:

```
- Samuel Chai: Adding a Renovate configuration to the repository.
- @kassett: Add the `gild ask` command ([#19](https://github.com/el-dorado-ci/gild/pull/19))
```

### --online

`gild generate --online` fills those gaps by asking the forge. GitHub knows every verified email address on an
account, so it resolves authors that git alone cannot, and it can find a pull request even when the commit
subject never mentioned one:

```
- @kassett: Adding a Renovate configuration to the repository. ([#4](https://github.com/el-dorado-ci/gild/pull/4))
- @kassett: Add the `gild ask` command ([#19](https://github.com/el-dorado-ci/gild/pull/19))
```

It is off by default, so generating a changelog stays offline and reproducible. Set `forge_api = true` in the
gild config to leave it on permanently, which is usually what you want in CI.

Lookups shell out to the `gh` CLI, so authentication is whatever `gh auth status` already reports and there is no
token for gild to manage. Requests are made only for what git could not answer on its own, so a repository whose
commits all carry noreply addresses and `(#N)` subjects makes none at all. If `gh` is missing, unauthenticated,
rate-limited or simply offline, gild falls back to the git-only output rather than failing the command.
