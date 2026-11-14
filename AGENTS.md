# Games

Small games in Rust, one cargo workspace. Each game has its own folder and its own releases; `gamekit` holds what the
games share. This file holds the rules for the whole repository; a game's own `AGENTS.md` adds what is true for that
game only.

| Folder | Contents |
|---|---|
| `bounty/` | Bounty, a tribute to King's Bounty (1990). Crates under `bounty/crates/`; only the application may use graphics and sound; see `bounty/AGENTS.md` |
| `gamekit/` | what the games share around their rules: virtual screen, text and font, translations, a game's own files, the dedication, a game updating itself, developer aids. Only a game's application depends on it |
| `tools/` | scripts used by the hooks, CI and releases; `build-linux.sh` and `package.sh` take the game as their first argument |
| `docs/REPOSITORY.md` | GitHub settings, rulesets, CI and Dependabot |
| `docs/RELEASING.md` | how a game is released: tag, workflow, files, update channel, signing key |

## Rules for every change

- Code and config are ASCII only (`make ascii-check`, also a commit hook). Docs and `.ftl` files are UTF-8. A line that
  truly needs other characters ends with `ascii-ok`.
- Run `make lint` and `make test` before calling a change done. Hooks (`prek.toml`) run the same checks on commit, and
  CI runs the hooks.
- Markdown docs wrap at 120 columns: run `make docs` after editing them (`make docs-check` checks). Rust stays at
  rustfmt's 100.
- Commits and PR titles are Conventional Commits with the game or crate as scope (`feat(bounty): ...`); changelogs are
  generated from them. No trailers in commits or PRs (no Co-authored-by, no "Generated with").
- A pull request is squashed: its title and description become the commit on `main`. Write them as that commit message,
  see "Pull requests" below.
- Lints, edition, licence and the Rust version are set once: `[workspace.lints]` and `[workspace.package]` in the root
  `Cargo.toml`, and `mise.toml`. A crate takes them with `[lints] workspace = true` and `*.workspace = true`; it does
  not set its own.
- A dependency used by more than one crate is declared in `[workspace.dependencies]`. Shared code is a path dependency
  inside this repository: `deny.toml` refuses git dependencies.
- No crate is published to crates.io (`publish = false`). A release is a game's application, tagged `<game>-vX.Y.Z`.
- A crate without graphics never depends on one with graphics. Rules, computer players and tools must build and test on
  a machine with no window and no sound device.

## Dedication

The games are dedicated to the memory of Joseph Rivkind; the words are in `README.md`, in English and Ukrainian, and are
not to be reworded. Every game shows them on its first run and in its About screen, with `gamekit::dedication`, so that
no game carries a copy of its own.

## Pull requests

The title of a pull request becomes the subject line of the commit on `main` and its description the body, word for
word. So they are written as a commit message, for someone reading `git log` a year from now, not as a note to the
reviewer of today. `tools/check_pr_message.sh` checks the parts a script can check, as part of the
`Conventional PR title` check.

- Title: a Conventional Commit in the imperative (`feat(bounty): add the wait action`), at most 65 characters (GitHub
  adds the pull request number, and the line stays within 72), no full stop at the end.
- Description: what changed and why, and what the code does not show by itself: the reason for a choice, what was left
  out on purpose, numbers that justify the change. Markdown is fine, in the title too.
- Width: the description follows `.editorconfig` like any Markdown, lines up to 120 columns; only the title has the
  short limit. (GitHub rewraps the description at 72 when it writes the commit; that is its doing and needs no care.)
- Not in the description: checklists, HTML comments, template headings with nothing under them, trailers, the state of
  the pull request ("draft until ...", "CI is green", "all tests pass"), greetings.
- What is true only while the pull request is open (test runs, measurements made for the review, what it waits for) is
  told to the maintainer directly and written nowhere. No comments on a pull request to carry it: they add nothing the
  checks and the diff do not show.
- Dependabot writes its own descriptions; trim them to a sentence when merging.

## Shell scripts

Shell scripts (`tools/*.sh`) follow the [Google Shell Style Guide](https://google.github.io/styleguide/shellguide.html)
with the changes below; where they disagree, this section wins. `make shell` formats with `shfmt -i 2 -ci`, and the
`shell` hook runs shfmt and shellcheck on every commit. A new script starts with:

```bash
#!/usr/bin/env bash
#
# What the script does, and why when it is not obvious.

set -euf -o pipefail
export LC_ALL="C.UTF-8"
PS4='+ $(date -u +%Y-%m-%dT%H:%M:%S%z) ${BASH_SOURCE[0]##*/}:${LINENO}: '
```

and ends with `main "$@"`, an empty line and `# EOF` (a truncated file then shows as a missing marker).

- `#!/usr/bin/env bash`, not `#!/bin/bash`: macOS ships bash 3.2 there, and NixOS has no `/bin/bash`.
- `set -euf -o pipefail`: `-u` stops on an unset variable, `-f` turns globbing off so a path is used literally. Match
  patterns with `find` (`readarray -t files < <(find "${dir}" -maxdepth 1 -name "${pattern}")`), not `set +f`. Leave
  `-e` out only in a script that must go on after a failed command, with a comment saying why.
- `LC_ALL="C.UTF-8"`: sorting and character classes behave the same on a runner and a laptop.
- `PS4` in single quotes, so every `set -x` line shows its own time, file and line.
- Functions: `function kebab-name() {`. The work goes in `main`. A script with several actions dispatches with
  `case "${1:-}"` in `main`, and its `*)` branch prints the usage to stderr and exits 1 (a bare `"$1"` would stop on
  "unbound variable" before the usage).
- Brace every expansion: `"${var}"`, not `"$var"`. Constants are `readonly`; `export` only what a child process needs.
- A required value gets a message: `"${1:?usage: ...}"` or `: "${VAR:?VAR must be set}"`.
- Errors go to stderr (`echo "..." >&2`).
- A continued pipeline ends its line with `|` (shfmt's layout). Lines up to 120 columns (`.editorconfig`).
- A script that guards something (like `check_ascii.sh`) is finished only once it has been seen failing on the input it
  exists to catch.

## Packaging

What is particular to a game's packages is in `<game>/packaging/`: `Info.plist` for the macOS bundle, `<game>.desktop`
for the AppImage, and the icon as PNGs in `icon/` (`<game>-16.png` to `<game>-1024.png`). The scripts in `tools/` do the
rest for any game, and name every file `<game>-vX.Y.Z-<system>-<arch>`. The CI build jobs build and package each game as
a release does, so a broken package shows in a pull request, not on release day.

A release is started by hand from its tag and published by `tools/publish.sh`; `docs/RELEASING.md` has the steps.

## Not here yet

- NixOS: `shell.nix` is copied from Scorch and has not been run from this repository; there is no flake yet.
- Releases: nothing has been released, so neither the workflow nor a game updating itself has met a real release.
  Changelogs and release pull requests are not set up (`docs/RELEASING.md`).

## Rejected Findings

None yet.
