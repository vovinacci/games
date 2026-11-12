# Games

Small games in Rust, one cargo workspace. Each game has its own folder and its own releases; `gamekit` (to come) holds
what the games share. This file holds the rules for the whole repository; a game's own `AGENTS.md` adds what is true for
that game only.

| Folder | Contents |
|---|---|
| `bounty/` | Bounty, a tribute to King's Bounty (1990). Crates under `bounty/crates/`; only the application may use graphics and sound |
| `tools/` | scripts used by the hooks and CI |
| `docs/REPOSITORY.md` | GitHub settings, rulesets, CI and Dependabot |

## Rules for every change

- Code and config are ASCII only (`make ascii-check`, also a commit hook). Docs and `.ftl` files are UTF-8. A line that
  truly needs other characters ends with `ascii-ok`.
- Run `make lint` and `make test` before calling a change done. Hooks (`prek.toml`) run the same checks on commit, and
  CI runs the hooks.
- Markdown docs wrap at 120 columns: run `make docs` after editing them (`make docs-check` checks). Rust stays at
  rustfmt's 100.
- Commits and PR titles are Conventional Commits with the game or crate as scope (`feat(bounty): ...`); changelogs are
  generated from them. No trailers in commits or PRs (no Co-authored-by, no "Generated with").
- Lints, edition, licence and the Rust version are set once: `[workspace.lints]` and `[workspace.package]` in the root
  `Cargo.toml`, and `mise.toml`. A crate takes them with `[lints] workspace = true` and `*.workspace = true`; it does
  not set its own.
- A dependency used by more than one crate is declared in `[workspace.dependencies]`. Shared code is a path dependency
  inside this repository: `deny.toml` refuses git dependencies.
- No crate is published to crates.io (`publish = false`). A release is a game's application, tagged `<game>-vX.Y.Z`.
- A crate without graphics never depends on one with graphics. Rules, computer players and tools must build and test on
  a machine with no window and no sound device.

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

## Not here yet

- NixOS: no `shell.nix` or flake. They come with the first crate that opens a window; until then cargo runs directly.
- Releases: no release workflow. It comes with the first application that has something to ship.

## Rejected Findings

None yet.
