#!/usr/bin/env bash
#
# Fails if the given files contain non-ASCII characters. Code and config stay ASCII; on-screen text belongs in
# each game's assets/i18n/*.ftl. A line that really needs other characters (such as Ukrainian test data) ends with "ascii-ok".

set -euf -o pipefail
export LC_ALL="C.UTF-8"
PS4='+ $(date -u +%Y-%m-%dT%H:%M:%S%z) ${BASH_SOURCE[0]##*/}:${LINENO}: '

function main() {
  local status=0 file
  for file in "$@"; do
    # In the C locale every byte above 127 is neither printable nor space. Not grep -P: macOS grep has no -P, fails
    # on it, and the check would then pass whatever the file holds.
    if LC_ALL=C grep -n '[^[:print:][:space:]]' "${file}" | grep -v 'ascii-ok'; then
      echo "^ non-ASCII in ${file}" >&2
      status=1
    fi
  done
  return "${status}"
}

main "$@"

# EOF
