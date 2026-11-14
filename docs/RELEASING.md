# Releases

Files: `.github/workflows/release.yml`, `release-plz.toml`, `tools/build-linux.sh`, `tools/package.sh`,
`tools/publish.sh`, `packaging/`, `gamekit/assets/release-key.pub`.

A release is a tag `vX.Y.Z` on `main` with a GitHub Release; every file of it is named `bounty-vX.Y.Z-<system>-<arch>`.
No crate is published to crates.io. The process is the one Scorch has.

## How a release is made

Tool: [release-plz](https://release-plz.dev) (changelog by git-cliff).

1. Every merge of a `feat`, `fix` or `perf` change to `main` opens or updates a **release PR** (other types wait and
   ride along with the next release): the version in every crate's `Cargo.toml` and in `Cargo.lock` (`fix` = patch,
   `feat` = minor, breaking = minor while below 1.0) and the new `CHANGELOG.md` section.
2. Review the PR, polish the wording if needed, merge it. Merge it last: it releases what is on `main` at that moment,
   and each feature or fix merged before it updates its version and changelog.
3. release-plz tags `vX.Y.Z` and creates the GitHub Release with that changelog section as notes.
4. The same workflow then builds on the systems below, packages with `tools/package.sh`, and `tools/publish.sh` writes
   `SHA256SUMS`, signs it and uploads everything. It must be the same workflow: a tag made by a workflow does not start
   other workflows.
5. To rebuild the files of an existing release: Actions > Release > Run workflow, with its tag.

The release PR is opened with the secret `RELEASE_TOKEN`, a fine-grained personal access token (this repository only;
Contents and Pull requests: read and write). The built-in token will not do: the repository does not let Actions open
pull requests, and a pull request opened with it would not start its checks.

Tags matching `v*` cannot be moved or deleted (`docs/REPOSITORY.md`): a wrong release is followed by the next version,
not repaired.

### One version for all crates

What is released is the application, `bounty-app`, but most changes land in the crates it is built from. release-plz
looks at one crate at a time, and left to that it either misses a feature in the rules or calls it a patch of the game.
So `release-plz.toml` puts all crates into one version group, measured against the same tag: a change in any of them
moves the version of all, by what the change is. Only `bounty-app` has a tag, a GitHub Release and a changelog, and its
changelog takes in the commits of the others. The crates' own version numbers mean nothing by themselves.

### The first release

There is no release 0.1.0. The tag `v0.1.0` was pushed while the crates still said 0.0.0, on a commit that would have
built a game calling itself 0.0.0, and tags cannot be moved; so it stays as the mark release-plz counts commits from,
and the first release is 0.1.1.

What that showed about release-plz: it reads the last release from the tag (its commit and the version in `Cargo.toml`
there), and when `Cargo.toml` on `main` already carries another version than the tagged commit, it proposes exactly that
version and counts nothing up. So a version is only ever set by the release PR. Setting it by hand, as was needed once
to get past 0.1.0, means the next release PR releases that number.

## Files of a release

| System | File |
|---|---|
| Linux x86_64, ARM64 | `bounty-vX.Y.Z-linux-<arch>.tar.gz` and `Bounty-vX.Y.Z-<arch>.AppImage` |
| Windows x86_64, ARM64 | `bounty-vX.Y.Z-windows-<arch>.zip` (no console window) |
| macOS (Intel and Apple Silicon) | `bounty-vX.Y.Z-macos.zip` with a universal `Bounty.app` |
| Linux, Windows (for the game updating itself) | `bounty-vX.Y.Z-linux-<arch>`, `bounty-vX.Y.Z-windows-<arch>.exe` |
| All | `SHA256SUMS` and its minisign signature `SHA256SUMS.minisig` |

- Linux builds link against a fixed glibc (2.35, `GLIBC_FLOOR` in `tools/build-linux.sh`) with cargo-zigbuild and fail
  if the program needs a newer one; ARM64 is built on GitHub's ARM runners. macOS builds set `MACOSX_DEPLOYMENT_TARGET`
  to 15.0.
- No paid code-signing certificate. Windows shows SmartScreen ("More info" > "Run anyway"). The macOS app has an ad-hoc
  signature (Apple Silicon refuses unsigned apps as "damaged"), and Gatekeeper still rejects it, so a player removes the
  quarantine mark with `xattr`.

## The game updating itself

`gamekit::update` does it; the game gives its name, repository and version and shows one line.

- **Check:** at every start of a released build, in the background: one request to
  `github.com/vovinacci/bounty-game/releases/latest`, whose redirect names the latest tag. A newer version that can be
  installed (on macOS the game has to run from `Bounty.app`) shows "Version X is out. U: update" at the foot of the
  sidebar; a check that fails says nothing. There is no question before the first check and no setting to turn it off
  yet: the game has no settings screen to hold the answer.
- **Update** (key U, or `bounty --update` without a window): `SHA256SUMS` of that release and its signature, verified
  with the public key built into the game, then this system's file, checked against its SHA-256. Only then is the game
  replaced, and U starts it again.
  - Linux and Windows: the plain program, swapped in with `self-replace` (which copes with a running program on
    Windows).
  - AppImage: the new AppImage replaces the file in `$APPIMAGE`.
  - macOS: the zip is unpacked next to `Bounty.app` with `ditto`, then the bundles are swapped. A bundle the game
    downloaded itself carries no quarantine mark.
- **Signing:** the release workflow signs `SHA256SUMS` with the minisign secret key in the secret `RELEASE_SIGNING_KEY`
  (unencrypted, as the workflow cannot type a password); a release without it fails. The public half is
  `gamekit/assets/release-key.pub`. Losing the key means players update by hand once, to a release with a new public
  key.
- **Off** for a build that is not a release (a debug build, version `0.0.0`, or one with `+`), for a developer's run
  (`--shot`, `--steps`), and without `gamekit`'s `self-update` feature, for a packager with updates of its own. HTTPS is
  rustls with Mozilla's root certificates built in: no OpenSSL, no certificate store of the system.

## Supply chain

- Every GitHub Action is pinned to a full commit SHA; the build tools (zig, cargo-zigbuild) are pinned with hashes in
  `requirements-build.txt`; appimagetool is a pinned release checked against its sha256.
- The workflow starts with no permissions and each job asks only for what it needs. Build jobs read the repository and
  keep no credentials and no caches; only the last job sees the signing key.
- The tag is checked against `vX.Y.Z` before anything is built, and reaches scripts through the environment, never
  pasted into a command.

## Not here yet

The game updating itself has been seen working on macOS only, with `bounty --update` from a released package (0.1.1 to
0.1.2). The plain program on Linux and Windows, the AppImage and the key in the window are untried.
