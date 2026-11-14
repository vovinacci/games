#!/usr/bin/env bash
#
# Packages the release build of the game into dist/ for the release workflow.
#
#   tools/package.sh linux   <arch> <version>   # target/release/bounty -> tar.gz, AppImage, plain program
#   tools/package.sh windows <arch> <version>   # target/release/bounty.exe -> zip, plain program
#   tools/package.sh macos   <version>          # universal Bounty.app from both Apple targets -> zip
#
# <arch> is x86_64 or aarch64; <version> is the version with its v, e.g. v0.1.0, so that every file is named
# bounty-vX.Y.Z-<os>-<arch>. What the packages need is in packaging/: Info.plist, bounty.desktop and the icon's PNGs. Needs appimagetool for the AppImage
# (APPIMAGETOOL, default "appimagetool"), 7z on Windows, and lipo/iconutil/ditto/codesign on macOS.

set -euf -o pipefail
export LC_ALL="C.UTF-8"
PS4='+ $(date -u +%Y-%m-%dT%H:%M:%S%z) ${BASH_SOURCE[0]##*/}:${LINENO}: '

readonly USAGE="usage: package.sh linux|windows <arch> <version> | macos <version>"
readonly GAME="bounty"

# The game's name as a title: bounty -> Bounty.
function title-of() {
  local game="$1"
  local first
  first="$(printf '%s' "${game:0:1}" | tr '[:lower:]' '[:upper:]')"
  printf '%s%s' "${first}" "${game:1}"
}

# Makes an empty dist/$2 with the documents of game $1 in it.
function stage() {
  local game="$1" name="$2"
  rm -rf "dist/${name}"
  mkdir -p "dist/${name}"
  cp README.md LICENSE "dist/${name}/"
}

function package-linux() {
  local game="$1"
  local arch="${2:?${USAGE}}"
  local version="${3:?${USAGE}}"
  local name="${game}-${version}-linux-${arch}"
  local title
  title="$(title-of "${game}")"
  stage "${game}" "${name}"
  cp "target/release/${game}" "dist/${name}/"
  tar -C dist -czf "dist/${name}.tar.gz" "${name}"
  # The plain program, for self-update (same name as the folder, so the folder goes first).
  rm -rf "dist/${name}"
  cp "target/release/${game}" "dist/${name}"

  local app="dist/${title}.AppDir"
  rm -rf "${app}"
  mkdir -p "${app}/usr/bin"
  cp "target/release/${game}" "${app}/usr/bin/"
  cp "packaging/${game}.desktop" "${app}/"
  cp "packaging/icon/${game}-256.png" "${app}/${game}.png"
  ln -s "usr/bin/${game}" "${app}/AppRun"
  ARCH="${arch}" APPIMAGE_EXTRACT_AND_RUN=1 "${APPIMAGETOOL:-appimagetool}" --no-appstream "${app}" \
    "dist/${title}-${version}-${arch}.AppImage"
  rm -rf "${app}"
}

function package-windows() {
  local game="$1"
  local arch="${2:?${USAGE}}"
  local version="${3:?${USAGE}}"
  local name="${game}-${version}-windows-${arch}"
  stage "${game}" "${name}"
  cp "target/release/${game}.exe" "dist/${name}/"
  (cd dist && 7z a -tzip -bso0 "${name}.zip" "${name}")
  rm -rf "dist/${name}"
  # The plain program, for self-update.
  cp "target/release/${game}.exe" "dist/${name}.exe"
}

function package-macos() {
  local game="$1"
  local version="${2:?${USAGE}}"
  local name="${game}-${version}-macos"
  local title
  title="$(title-of "${game}")"
  stage "${game}" "${name}"
  local app="dist/${name}/${title}.app"
  mkdir -p "${app}/Contents/MacOS" "${app}/Contents/Resources"
  lipo -create -output "${app}/Contents/MacOS/${game}" \
    "target/aarch64-apple-darwin/release/${game}" "target/x86_64-apple-darwin/release/${game}"
  sed "s/@VERSION@/${version#v}/" "packaging/Info.plist" >"${app}/Contents/Info.plist"

  local iconset="dist/${title}.iconset" size
  rm -rf "${iconset}"
  mkdir -p "${iconset}"
  for size in 16 32 128 256 512; do
    cp "packaging/icon/${game}-${size}.png" "${iconset}/icon_${size}x${size}.png"
    cp "packaging/icon/${game}-$((size * 2)).png" "${iconset}/icon_${size}x${size}@2x.png"
  done
  iconutil -c icns "${iconset}" -o "${app}/Contents/Resources/${title}.icns"
  rm -rf "${iconset}"

  # Ad-hoc signature: Apple Silicon refuses unsigned apps outright ("damaged"). Gatekeeper still rejects ad-hoc apps;
  # players clear the quarantine mark with xattr. Opening normally would need a Developer ID and notarization.
  codesign --force --deep --sign - "${app}"
  codesign --verify --deep --strict --verbose=2 "${app}"
  (cd dist && ditto -c -k --keepParent "${name}" "${name}.zip")
  rm -rf "dist/${name}"
}

function main() {
  mkdir -p dist
  case "${1:-}" in
    linux) package-linux "${GAME}" "${@:2}" ;;
    windows) package-windows "${GAME}" "${@:2}" ;;
    macos) package-macos "${GAME}" "${@:2}" ;;
    *)
      echo "${USAGE}" >&2
      exit 1
      ;;
  esac
}

main "$@"

# EOF
