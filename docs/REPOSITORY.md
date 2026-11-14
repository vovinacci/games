# GitHub, CI and repository settings

Files: `.github/workflows/ci.yml`, `release.yml`, `pr-title.yml`, `.github/dependabot.yml`, the issue templates. There
is no pull request template on purpose: whatever it held would end up in every commit message. The settings below are
made by hand (or `gh api`); this page is their record. Change the setting and this page together.

## Repository

- `vovinacci/games`, public, MIT. Issues on; wiki, projects and discussions off.
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
- `release tags`: tags matching `*-v*` (`<game>-vX.Y.Z`) can't be moved or deleted.
- Repository admins can bypass both, for emergencies. A new required check is added to the `main` ruleset in the same
  change that adds its job, or the job protects nothing.

## CI (every push to `main` and every pull request)

- `Checks`: all prek hooks (whitespace, ASCII, Markdown, shell, rustfmt, clippy), then `cargo test` for the whole
  workspace with the committed lockfile.
- `Build (Linux x86_64)`, `Build (Windows x86_64)`, `Build (macOS)`: a release build of each game's application.
- `Dependencies`: cargo-deny (advisories, licences, duplicates, sources) as set in `deny.toml`.
- `Conventional PR title`: the title must be a Conventional Commit, and title and description must pass
  `tools/check_pr_message.sh`, since they become the commit on `main` (`AGENTS.md`, "Pull requests").
- Rust comes from `mise.toml` via `jdx/mise-action`; dependencies are cached (`Swatinem/rust-cache`).

## Dependabot

Weekly, grouped, with a seven-day cooldown: Cargo (`build:`), the Python tools in `requirements.txt` (`build:`) and
GitHub Actions (`ci:`).

## Releases

`docs/RELEASING.md`: the `Release` workflow, the files of a release, the update channel and the signing key. The
repository secret `RELEASE_SIGNING_KEY` holds the minisign secret key.
