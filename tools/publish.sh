#!/usr/bin/env bash
#
# Publishes the packages of one release of a game: checksums, their signature, the files on the GitHub release, and
# then the game's update channel.
#
#   tools/publish.sh <game> <version> <dir>   # e.g. bounty v0.1.0 dist
#
# <dir> holds the packages and nothing else. The script writes SHA256SUMS over them, signs it with minisign and
# uploads everything to the release <game>-<version>, which it creates if the tag has none yet. Last of all it
# points the channel at the release: the fixed release <game>-channel holds latest.txt, one line with the tag of
# the newest release, and its signature. A game asks the channel what is newest, never GitHub's "latest", which
# knows nothing of games. The channel only moves forward, so rebuilding an old release leaves it alone.
#
# Needs gh (GH_TOKEN, GH_REPO), minisign, and the unencrypted minisign secret key in SIGNING_KEY; the public half
# is gamekit/assets/release-key.pub. GITHUB_SHA is where the channel's tag is put when the channel is first made.

set -euf -o pipefail
export LC_ALL="C.UTF-8"
PS4='+ $(date -u +%Y-%m-%dT%H:%M:%S%z) ${BASH_SOURCE[0]##*/}:${LINENO}: '

readonly USAGE="usage: publish.sh <game> <version> <dir>"

# The game's name as a title: bounty -> Bounty.
function title-of() {
  local game="$1"
  local first
  first="$(printf '%s' "${game:0:1}" | tr '[:lower:]' '[:upper:]')"
  printf '%s%s' "${first}" "${game:1}"
}

# Signs file $1 with the comment $2; the signature is $1.minisig. The comment is signed too.
function sign() {
  minisign -S -s "${KEY_FILE}" -m "$1" -t "$2"
}

# Uploads the packages in the current directory, with their signed checksums, to the release of tag $1.
function publish-packages() {
  local tag="$1" name="$2"
  local -a files
  readarray -t files < <(find . -maxdepth 1 -type f | sed 's|^\./||' | sort)
  if [[ "${#files[@]}" -eq 0 ]]; then
    echo "no packages to publish" >&2
    exit 1
  fi
  sha256sum -- "${files[@]}" >SHA256SUMS
  sign SHA256SUMS "${tag}"
  if ! gh release view "${tag}" >/dev/null 2>&1; then
    gh release create "${tag}" --verify-tag --latest=false --title "${name}" --notes ""
  fi
  gh release upload --clobber "${tag}" -- "${files[@]}" SHA256SUMS SHA256SUMS.minisig
}

# Points the channel of game $1 at tag $2, unless it already names a newer one. Works in the empty directory $4.
function publish-channel() {
  local game="$1" tag="$2" title="$3" work="$4"
  local channel="${game}-channel" assets current="" newest
  if ! gh release view "${channel}" >/dev/null 2>&1; then
    gh release create "${channel}" --target "${GITHUB_SHA:?GITHUB_SHA must be set}" --latest=false \
      --title "${title}: update channel" \
      --notes "Not a release. latest.txt names the newest release of ${title}; the game reads it to find updates."
  else
    assets="$(gh release view "${channel}" --json assets --jq '.assets[].name')"
    if grep -qx 'latest.txt' <<<"${assets}"; then
      current="$(gh release download "${channel}" --pattern latest.txt --output -)"
    fi
  fi
  newest="$(printf '%s\n%s\n' "${current}" "${tag}" | sort -V | tail -n 1)"
  if [[ "${newest}" != "${tag}" ]]; then
    echo "channel ${channel} stays at ${current}, which is newer than ${tag}"
    return
  fi
  printf '%s\n' "${tag}" >"${work}/latest.txt"
  sign "${work}/latest.txt" "${tag}"
  # The signature first: a game that reads between the two uploads sees an old name with a new signature, fails
  # the check and asks again later; it never sees a name it would act on without its signature.
  gh release upload --clobber "${channel}" "${work}/latest.txt.minisig"
  gh release upload --clobber "${channel}" "${work}/latest.txt"
  echo "channel ${channel} now names ${tag}"
}

function main() {
  local game="${1:?${USAGE}}" version="${2:?${USAGE}}" dir="${3:?${USAGE}}"
  : "${SIGNING_KEY:?SIGNING_KEY must be set: a game only updates itself from a signed release}"
  if [[ ! "${version}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "not a version: ${version}" >&2
    exit 1
  fi
  local title tag="${game}-${version}"
  title="$(title-of "${game}")"
  # Not local: the trap runs after main has returned.
  WORK="$(mktemp -d)"
  readonly WORK
  readonly KEY_FILE="${WORK}/release.key"
  trap 'rm -rf "${WORK}"' EXIT
  (umask 077 && printf '%s\n' "${SIGNING_KEY}" >"${KEY_FILE}")
  mkdir "${WORK}/channel"

  cd "${dir}"
  publish-packages "${tag}" "${title} ${version#v}"
  publish-channel "${game}" "${tag}" "${title}" "${WORK}/channel"
}

main "$@"

# EOF
