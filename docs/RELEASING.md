# Releases

Files: `.github/workflows/release.yml`, `tools/build-linux.sh`, `tools/package.sh`, `tools/publish.sh`, `packaging/`,
`gamekit/assets/release-key.pub`.

Each game is released on its own. A release is a tag `<game>-vX.Y.Z` on `main`, and every file of it carries that name
with the system after it. No crate is published to crates.io.

## Making a release

1. Set the version in the game's application (`crates/bounty-app/Cargo.toml`, and `Cargo.lock` with it) in a pull
   request, like any change. The game shows this version on screen.
2. Tag the merged commit and push the tag. The tag is signed, as commits are:

   ```sh
   git fetch origin
   git tag -s bounty-v0.1.0 -m "Bounty 0.1.0" origin/main
   git push origin bounty-v0.1.0
   ```

3. Start the workflow with the tag: Actions > Release > Run workflow, or

   ```sh
   gh workflow run release.yml --repo vovinacci/games -f tag=bounty-v0.1.0
   ```

The workflow builds on the systems below, packages with `tools/package.sh`, and hands the files to `tools/publish.sh`,
which writes `SHA256SUMS`, signs it, creates the GitHub release for the tag if there is none and uploads everything.
Then it moves the game's update channel. Running the workflow again for the same tag rebuilds and replaces the files of
that release.

Tags matching `*-v*` cannot be moved or deleted (`docs/REPOSITORY.md`): a wrong release is followed by the next version,
not repaired.

## Files of a release

| System | File |
|---|---|
| Linux x86_64, ARM64 | `<game>-vX.Y.Z-linux-<arch>.tar.gz` and `<Game>-vX.Y.Z-<arch>.AppImage` |
| Windows x86_64, ARM64 | `<game>-vX.Y.Z-windows-<arch>.zip` (no console window) |
| macOS (Intel and Apple Silicon) | `<game>-vX.Y.Z-macos.zip` with a universal `<Game>.app` |
| Linux, Windows (for the game updating itself) | `<game>-vX.Y.Z-linux-<arch>`, `<game>-vX.Y.Z-windows-<arch>.exe` |
| All | `SHA256SUMS` and its minisign signature `SHA256SUMS.minisig` |

- Linux builds link against a fixed glibc (2.35, `GLIBC_FLOOR` in `tools/build-linux.sh`) with cargo-zigbuild and fail
  if the program needs a newer one; ARM64 is built on GitHub's ARM runners. macOS builds set `MACOSX_DEPLOYMENT_TARGET`
  to 15.0.
- No paid code-signing certificate. Windows shows SmartScreen ("More info" > "Run anyway"). The macOS app has an ad-hoc
  signature (Apple Silicon refuses unsigned apps as "damaged"), and Gatekeeper still rejects it, so a player removes the
  quarantine mark with `xattr`.

## Update channel

GitHub marks one release of a repository as "latest", and knows nothing of games: with two games here it would name
whichever was released last. So no game asks for it, and no release is marked latest. Each game has a fixed release
instead, `<game>-channel`, which is not a release of anything and holds two files:

- `latest.txt`: one line, the tag of the game's newest release (`bounty-v0.1.0`);
- `latest.txt.minisig`: its minisign signature.

Their addresses never change: `https://github.com/vovinacci/games/releases/download/<game>-channel/latest.txt`.

`tools/publish.sh` writes them after the files of a release are uploaded, so the channel never names a release that is
not complete, and only when the tag is newer than the one already there, so rebuilding an old release does not send
players back. The first release of a game creates its channel.

## Signing

One minisign key signs for every game in the repository. The secret half is the repository secret `RELEASE_SIGNING_KEY`
(unencrypted, as the workflow cannot type a password); a release without it fails. The public half is
`gamekit/assets/release-key.pub`, built into each game.

- The key says "published by this repository", not "this game" or "this version". What a signature covers says the rest:
  `latest.txt` holds the tag with the game's name in it, and `SHA256SUMS` lists files named after game and version. A
  game must check those names, and take only a version newer than its own.
- Losing the key means players update by hand once, to a release that carries a new public key.

## Supply chain

- Every GitHub Action is pinned to a full commit SHA; the build tools (zig, cargo-zigbuild) are pinned with hashes in
  `requirements-build.txt`; appimagetool is a pinned release checked against its sha256.
- The workflow starts with no permissions. Build jobs read the repository and keep no credentials and no caches; only
  the last job may write releases, and only it sees the signing key.
- The tag is checked against `<game>-vX.Y.Z` before anything is built, and reaches scripts through the environment,
  never pasted into a command.

## Not here yet

- The game does not yet look at its channel; the updater comes next, in `gamekit`.
- Release notes: a release is created with empty notes. A changelog written from the Conventional Commits, and a release
  pull request that sets the version (release-plz, as Scorch has), are not set up.
