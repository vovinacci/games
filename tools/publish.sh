#!/usr/bin/env bash
#
# Publishes the packages of one release: checksums, their signature, and the files on the GitHub release.
#
#   tools/publish.sh <tag> <dir>   # e.g. v0.1.0 dist
#
# <dir> holds the packages and nothing else. The script writes SHA256SUMS over them, signs it with minisign and
# uploads everything to the release of <tag>, which it creates if the tag has none yet (release-plz normally has
# made it, with the notes).
#
# Needs gh (GH_TOKEN, GH_REPO), minisign, and the unencrypted minisign secret key in SIGNING_KEY; the public half
# is gamekit/assets/release-key.pub, with which the game checks SHA256SUMS before it updates itself.

set -euf -o pipefail
export LC_ALL="C.UTF-8"
PS4='+ $(date -u +%Y-%m-%dT%H:%M:%S%z) ${BASH_SOURCE[0]##*/}:${LINENO}: '

readonly USAGE="usage: publish.sh <tag> <dir>"

function main() {
  local tag="${1:?${USAGE}}" dir="${2:?${USAGE}}"
  : "${SIGNING_KEY:?SIGNING_KEY must be set: the game only updates itself from a signed release}"
  if [[ ! "${tag}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "not a release tag: ${tag}" >&2
    exit 1
  fi
  # Not local: the trap runs after main has returned.
  WORK="$(mktemp -d)"
  readonly WORK
  trap 'rm -rf "${WORK}"' EXIT
  (umask 077 && printf '%s\n' "${SIGNING_KEY}" >"${WORK}/release.key")

  cd "${dir}"
  local -a files
  # Not the checksums of an earlier, interrupted run.
  readarray -t files < <(find . -maxdepth 1 -type f ! -name 'SHA256SUMS*' | sed 's|^\./||' | sort)
  if [[ "${#files[@]}" -eq 0 ]]; then
    echo "no packages to publish" >&2
    exit 1
  fi
  sha256sum -- "${files[@]}" >SHA256SUMS
  # The comment is signed too.
  minisign -S -s "${WORK}/release.key" -m SHA256SUMS -t "Bounty ${tag}"
  if ! gh release view "${tag}" >/dev/null 2>&1; then
    gh release create "${tag}" --verify-tag --title "Bounty ${tag#v}" --notes ""
  fi
  gh release upload --clobber "${tag}" -- "${files[@]}" SHA256SUMS SHA256SUMS.minisig
}

main "$@"

# EOF
