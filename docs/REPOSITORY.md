# GitHub, CI and repository settings

Files: `.github/workflows/ci.yml`, `release.yml`, `pr-title.yml`, `.github/dependabot.yml`, `release-plz.toml`, the
issue templates. There is no pull request template on purpose: whatever it held would end up in every commit message.
The settings below are made by hand (or `gh api`); this page is their record. Change the setting and this page together.

## Repository

- `vovinacci/bounty-game`, public, MIT (the game and its program stay "Bounty" / `bounty`). Issues on; wiki, projects
  and discussions off.
- Squash merges only; the squash commit takes the PR title and body, so the PR title becomes the commit on `main`.
  Branches are deleted after merge.
- Secret scanning with push protection, Dependabot alerts and security updates, and private vulnerability reporting
  (`SECURITY.md`) are on.
- Actions: workflows get a read-only token by default and ask for more per job; every action must be pinned to a full
  commit SHA (a setting, so an unpinned action fails the run); Actions may not create or approve pull requests.

## Rulesets

- `main`: needs a pull request with the checks `Checks`, the three `Build` jobs, `Dependencies` and
  `Conventional PR title` passing, resolved review threads, linear history and signed commits; it can't be deleted or
  force-pushed.
- `release tags`: tags matching `v*` can't be moved or deleted.
- Repository admins can bypass both, for emergencies. A new required check is added to the `main` ruleset in the same
  change that adds its job, or the job protects nothing.

## CI (every push to `main` and every pull request)

- `Checks`: all prek hooks (whitespace, ASCII, Markdown, shell, rustfmt, clippy), then `cargo test` for the whole
  workspace with the committed lockfile.
- `Build (Linux x86_64)`, `Build (Windows x86_64)`, `Build (macOS)`: the game built and packaged as a release is.
- `Dependencies`: cargo-deny (advisories, licences, duplicates, sources) as set in `deny.toml`.
- `Conventional PR title`: the title must be a Conventional Commit, and title and description must pass
  `tools/check_pr_message.sh`, since they become the commit on `main` (`AGENTS.md`, "Pull requests").
- Rust comes from `mise.toml` via `jdx/mise-action`; dependencies are cached (`Swatinem/rust-cache`).

## Dependabot

Weekly, grouped, with a seven-day cooldown: Cargo (`build:`), the Python tools in `requirements.txt` (`build:`) and
GitHub Actions (`ci:`).

## Releases

`docs/RELEASING.md`: release-plz, the `Release` workflow, the files of a release and the game updating itself. Two
repository secrets: `RELEASE_SIGNING_KEY`, the minisign secret key, and `RELEASE_TOKEN`, the personal access token the
release PR is opened with.
