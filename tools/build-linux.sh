#!/usr/bin/env bash
#
# Release build of the game for Linux that runs on any distribution with glibc GLIBC_FLOOR or newer, whatever the build
# machine has: cargo-zigbuild links against that glibc version (tools pinned in requirements-build.txt). Then checks
# the program and fails if it needs a newer glibc. The result goes to target/release/bounty, where tools/package.sh
# expects it.
#
#   tools/build-linux.sh   # needs python3 and objdump
#
# The game plays no sound yet. When it does, ALSA has to be linked here the way Scorch's script of the same name
# does it: only libasound offered to the linker, never the build machine's own glibc.

set -euf -o pipefail
export LC_ALL="C.UTF-8"
PS4='+ $(date -u +%Y-%m-%dT%H:%M:%S%z) ${BASH_SOURCE[0]##*/}:${LINENO}: '

readonly GLIBC_FLOOR="2.35"
readonly GAME="bounty"

# Installs zig and cargo-zigbuild into a venv under $1 and puts them on PATH.
function install-zigbuild() {
  local tmp="$1"
  python3 -m venv "${tmp}/zigbuild"
  "${tmp}/zigbuild/bin/pip" install --quiet --disable-pip-version-check --require-hashes -r requirements-build.txt
  export PATH="${tmp}/zigbuild/bin:${PATH}"
}

# Fails if the program $1 needs a glibc newer than GLIBC_FLOOR.
function check-glibc() {
  local binary="$1"
  local needed
  needed="$(objdump -T "${binary}" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)"
  echo "${binary##*/} needs glibc ${needed} (floor ${GLIBC_FLOOR})"
  if [[ "$(printf '%s\n%s\n' "${needed}" "${GLIBC_FLOOR}" | sort -V | tail -1)" != "${GLIBC_FLOOR}" ]]; then
    echo "${binary##*/} needs glibc ${needed}, newer than the promised ${GLIBC_FLOOR}" >&2
    exit 1
  fi
}

function main() {
  local target tmp out
  target="$(uname -m)-unknown-linux-gnu"
  tmp="${RUNNER_TEMP:-$(mktemp -d)}"
  out="${CARGO_TARGET_DIR:-target}"

  install-zigbuild "${tmp}"
  cargo zigbuild --release --locked -p "${GAME}-app" --target "${target}.${GLIBC_FLOOR}"
  check-glibc "${out}/${target}/release/${GAME}"

  mkdir -p "${out}/release"
  cp "${out}/${target}/release/${GAME}" "${out}/release/${GAME}"
}

main "$@"

# EOF
