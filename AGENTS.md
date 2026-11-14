# Bounty

A tribute to King's Bounty (1990, DOS), in Rust, one cargo workspace. This file holds the rules for every change; the
specification is in `docs/spec/`, and `README.md` gives the reading order.

| Folder | Contents |
|---|---|
| `crates/` | `bounty-core` (rules and state), `bounty-ai` (computer players), `bounty-tools` (developer tools), `bounty-app` (the game: window, keyboard, drawing). Only the application may use graphics and sound |
| `gamekit/` | what a game needs around its rules, kept apart so that another game can take it: virtual screen, text and font, translations, the game's own files, the dedication, the game updating itself, developer aids. Only the application depends on it |
| `content/`, `prototype/`, `balance/`, `schemas/` | rules and expeditions as data, the engine's test fixtures, the fixed cases with their baselines, JSON schemas |
| `packaging/` | what the packages need: `Info.plist`, desktop entry, icon |
| `tools/` | scripts used by the hooks, CI and releases |
| `docs/spec/` | the specification |
| `docs/REPOSITORY.md` | GitHub settings, rulesets, CI and Dependabot |
| `docs/RELEASING.md` | how a release is made: release-plz, workflow, files, signing key, the game updating itself |

## Rules for every change

- Code and config are ASCII only (`make ascii-check`, also a commit hook). Docs and `.ftl` files are UTF-8. A line that
  truly needs other characters ends with `ascii-ok`.
- Run `make lint` and `make test` before calling a change done. Hooks (`prek.toml`) run the same checks on commit, and
  CI runs the hooks.
- Markdown docs wrap at 120 columns: run `make docs` after editing them (`make docs-check` checks). Rust stays at
  rustfmt's 100.
- Commits and PR titles are Conventional Commits, with the crate or part as scope where it helps (`feat(bounty): ...`,
  `fix(gamekit): ...`); changelogs are generated from them. No trailers in commits or PRs (no Co-authored-by, no
  "Generated with").
- A pull request is squashed: its title and description become the commit on `main`. Write them as that commit message,
  see "Pull requests" below.
- Lints, edition, licence and the Rust version are set once: `[workspace.lints]` and `[workspace.package]` in the root
  `Cargo.toml`, and `mise.toml`. A crate takes them with `[lints] workspace = true` and `*.workspace = true`; it does
  not set its own.
- A dependency used by more than one crate is declared in `[workspace.dependencies]`. Shared code is a path dependency
  inside this repository: `deny.toml` refuses git dependencies.
- No crate is published to crates.io (`publish = false`). A release is the application, tagged `vX.Y.Z`.
- A crate without graphics never depends on one with graphics. Rules, computer players and tools must build and test on
  a machine with no window and no sound device.

## Rules of the game

- Rebuild the 1990 DOS game, not a later King's Bounty or a Heroes of Might and Magic hybrid. No speed-based turn order
  unless the rules register records evidence for it.
- Only `bounty-core` changes game state. The application, the computer players and the tools submit commands to it. No
  macroquad, window, sound or clock in `bounty-core`, `bounty-ai` or `bounty-tools`.
- A refused command changes nothing: no resources, no state revision, no random draw.
- Rules use integers and a fixed order. No floats, and no iteration over a hash map, where the result decides an
  outcome.
- Random numbers for rules, for a computer player's planning and for presentation come from separate generators; a
  computer player never sees the rules' generator.
- `content/meadow/` is this project's own scenario on the reference's troops, for measuring the campaign's rules with
  the expedition pilot `errand-v1`. Its map and numbers are not the original's, and are never quoted as such.
- `prototype/` and `content/demo/` test the engine. Never tune a computer player on them or quote them as the original's
  balance, and never use their numbers where a number of the original is missing.
- A rule taken from documentation or another reimplementation is recorded in `docs/spec/07-RULES-REGISTER.md` with its
  source and status. Never present it as observed in the original.
- The reference rules (`content/reference/`) never change for the sake of better play. A rule change is a switch of the
  variant `content/modern/`, measured before and after (`docs/spec/10-GAMEPLAY-GAPS.md`), and adopted or dropped by its
  numbers and by play.
- A change to rules, data or a computer player comes with a before/after run of the fixed cases (`balance/`) and a
  sentence on what moved and why. The old baseline stays in git; a missing baseline is not a pass.
- A change in behaviour updates the requirement, its data and its test together (`docs/spec/`, IDs like `G-CMB-005`,
  `AT-013`).
- Files of the original game, anything extracted from them, original saves and captures never enter the repository or a
  release.
- Report only checks that were run.

## Measuring a change

`make balance` plays the cases of every suite in `balance/` (prototype, reference, the `modern` variant) with every seed
and the routes of the expeditions (the demo's, and the meadow's under the reference rules and the variant), and writes a
report for each under `target/` (`BALANCE_ARGS="--case melee_exchange"` picks one case). Compare two reports with

```sh
cargo run -q -p bounty-tools -- balance compare --baseline <before.json> --candidate <after.json>
```

and add `--exact` for a change that must not move any result. `make balance-check` does that against the recorded
baseline, and so does a test, so CI fails when a battle of the suite ends differently or a route of the expedition costs
something else (`bounty-tools expedition run|compare`; time, gold in and out, losses). Replacing the baseline:
`balance/README.md`. A report with problems (a refused command, an outcome the case does not allow, a run that does not
repeat) is a defect to fix, not a result.

## The application

- All text on screen goes through `t!("key")`, with the key in both `crates/bounty-app/assets/i18n/en-GB.ftl` (British
  spelling) and `uk.ftl`. Never hard-code on-screen text; tests check the keys. A stack is always named with its side
  and in one form ("your Wardens", "ваші Вартові"), and sentences are built so that the name is never declined.
- The screen offers only what `Battle::legal_actions` lists (`control.rs`); it has no rules of its own. On the map it
  asks `Expedition::refusal` before it offers a key, and a command refused all the same is told in the log in the
  player's language (`words::refusal_text`).
- Without flags the game is the expedition: `adventure.rs` holds its state, log and save and reads no key, so tests play
  it; `map.rs` draws the map, `view.rs` the battle, alone or inside the expedition (`input.rs` reads both). It is saved
  after every accepted command and continued at the next start; `--rules` or `--scenario` alone gives one battle
  instead. `--expedition demo|meadow` picks the expedition, `meadow` under `--rules reference` or `modern`.
- `--rules reference` plays under the reference rules (battles in `content/reference/battles.json`), `--rules modern`
  under the variant on the same battles; without it the prototype's. The header always says which rules are in use.
- What the rules do to a stack is told where it is shown: its morale with the stacks that lower it, and how many of it
  fight where the leadership does not reach all (`Battle::morale_lowered_by`, `Battle::fighting`). The screen asks the
  battle; it works nothing out by itself.
- Check a changed screen in both languages by screenshot, not by eye on a window:
  `cargo run -p bounty-app -- --lang uk --scenario flight_barrier --steps 3 --shot shot.png`. `--steps` lets the pilot
  play that many commands first; in the expedition it plays that many commands of a route of the fixture
  (`--route <id>`, `complete-route` unless named; 5 stands in the first battle, 99 at the end). With `--shot` or
  `--steps` the user's files are neither read nor written.
- Ukrainian runs longer than English: text with a fixed place uses `text_fit` or `text_wrapped` from `gamekit`.
- A run without `--shot` reads and writes `settings.txt` and, in an expedition, `save-<expedition>-<rules>.json` with
  its `.previous` in the user's data folder: each expedition under each rules has its own save. The user may be playing:
  stop only your own process, by its ID.

## Saving

- A save is the envelope of `schemas/save-envelope.schema.json`, written by `bounty-core::save`: the expedition and the
  battle being fought, in the canonical encoding, with a checksum, the rules' ID and a hash of the three content files.
  `decode` gives an expedition or one reason (`SaveError`): not a save, another version, damaged, other content, or a
  state the rules cannot reach. Nothing is half loaded.
- A new field of `Expedition` or `Battle` goes into `to_canon`, `from_canon` and `check` together; the round-trip tests
  (`save/tests.rs`, `bounty-ai/tests/save_battles.rs`, `expedition_routes.rs`) fail when one is forgotten. A change to
  what is written is a new `SCHEMA_VERSION`; there is no migration yet, an older save is refused by its version.
- The application (`saves.rs`) writes a save with `gamekit::files::write_keeping_previous` and reads it with
  `read_newest`: one current file and the one before it. A file that cannot be read is never deleted or overwritten.

## Dedication

The game is dedicated to the memory of Joseph Rivkind; the words are in `README.md`, in English and Ukrainian, and are
not to be reworded. The game shows them on its first run, with `gamekit::dedication`, which holds the words so that
another game taking the kit carries the same ones.

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

What the packages need is in `packaging/`: `Info.plist` for the macOS bundle, `bounty.desktop` for the AppImage, and the
icon as PNGs in `icon/` (`bounty-16.png` to `bounty-1024.png`). The scripts in `tools/` do the rest and name every file
`bounty-vX.Y.Z-<system>-<arch>`. The CI build jobs build and package the game as a release does, so a broken package
shows in a pull request, not on release day.

Releases are made by release-plz from the Conventional Commits; all crates carry one version, and only the application
is tagged. `docs/RELEASING.md` has the steps and the reasons.

## Not here yet

- Sound, a menu.
- Most of the campaign. Two small expeditions are whole in the rules and on screen: map, clock and weekly budget,
  recruiting, contract, battles with their reward and clue, surrender and recovery, search, save and continue.
  Dwellings, castles, spells on the map, the original's world: not yet.
- NixOS: `shell.nix` is copied from Scorch and has not been run from this repository; there is no flake yet.
- Releases: nothing has been released, so neither the workflow nor the game updating itself has met a real release.

## Rejected Findings

None yet.
