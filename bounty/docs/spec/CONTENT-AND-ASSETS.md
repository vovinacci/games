# Content, importing and presentation

## Proposed product architecture

Public engine + original procedural demo; optional local-data adapter for one selected DOS edition. The player
explicitly selects the source directory. The adapter reads it without modifying it and converts supported content into
the engine's validated internal types. A private cache avoids repeated decoding. OpenKB explicitly supports reading
original files [S8]; OpenBounty documents an extractor, but also contains port-authored constants [S9]. Therefore
extracting files is not the same as recovering every original algorithm.

No generic pack/plugin platform, automatic downloader, bundled DOS executable, cloud import, or upload of the user's
assets is required.

## Adapter contract

Input: a user-selected directory and a selected supported edition descriptor. Detection computes SHA-256 for required
source files and uses an explicit edition signature. Exact filenames, offsets, compression and archive layouts belong to
the edition adapter; they are not guessed from a familiar filename. Unknown or ambiguous editions fail with a useful
unsupported-edition error.

Output: `ImportedContent` (definitions, map layouts, location/string references, and optional decoded images), plus
provenance containing source file hashes, adapter version and a list of extracted versus port-defined fields. Each used
field must have valid bounds and referential integrity. Executable rules remain versioned Rust policy, not a magical
consequence of importing a troop table.

`content/import-policy.json` intentionally has no approved source signatures yet. The user's edition was not supplied. A
first probe should verify a few troop records and map landmarks before decoding a complete catalogue. Do not claim
supported content when an unknown field was silently replaced by a prototype value.

Imported graphics are optional. The first playable adapter may provide only statistics/maps while procedural symbols
render them. Audio extraction is not a first-milestone dependency. A converted cache is still user-supplied content, not
an automatically redistributable open-source asset.

Use bounded decoders, checked offsets and lengths; reject traversal, absolute archive entry paths and decompression
bombs. Cap individual source files at 64 MiB and total decoded content at 256 MiB for the initial adapter, with
documented configuration if a legitimate supported edition requires a larger limit. Never execute KB.EXE as part of
import. No original bytes are logged or uploaded by CI.

## Public/private distribution boundary

Public allowlist: authored Rust/tools/specs; original synthetic scenarios and test vectors; authored procedural
drawing/audio code; specifically approved third-party libraries/assets with their required notices. Do not treat "not
copied sprites" as equivalent to "nothing protected exists": maps, text, audio and code need their own provenance
decisions. Names and titles also raise a different trademark question from copyright [S13, S14]. No particular trademark
status is asserted.

Keep source DOS files, extracted caches, original saves and captures outside the repository. Local `private/` or
`imported/` folders, if needed, must be ignored, and release assembly must use an allowlist rather than zip the entire
working tree. CI uses fabricated parser inputs and the original demo only. Local integration checks may use the user's
copy and are explicitly absent from public CI coverage.

The game is named Bounty (folder `bounty`, crates `bounty-*`), chosen by the maintainer; the original's title appears
only in the phrase "a tribute to King's Bounty (1990)". The name has not been searched for conflicting games or marks.
This engineering policy is not legal clearance for copying, reverse engineering, distribution or trademark use in any
jurisdiction; relevant license/jurisdiction questions remain separate. A project being free does not establish
permission.

## Art plan: no surprise asset-production project

M1: simple board and unit tokens. Each troop receives a generated role silhouette (melee/ranged/flying), stable short
label and visible count/health. Unknown troop art uses its stable ID label rather than disappearing. Selection and
ownership use shape/outline as well as colour. Death removes the token with an optional brief fade; attacks use a
line/flash. The first release does not need individual portraits for every troop, villain or artefact.

M2: procedural water/plain/forest/blocked-ground tiles; settlement, encounter, clue and objective markers. A tiny icon
function library is sufficient. Terrain properties come from content, not from colour comparisons. Later imported
sprites may replace drawing without changing rules or collision. Nearest sampling is fixed when pixel sprites are used;
a filter-settings screen is deferred.

Audio: six authored short cues (confirm, cancel, move, hit, victory, defeat), produced by simple oscillator/envelope
functions or equivalent generated WAVs. No original melodies or samples, no music production requirement. Audio
generation and playback live in the app and never consume gameplay RNG; headless tests do not initialise an audio
device. A mute switch is enough initially.

## Fixed initial layout

Proposed remake canvas: 1280 x 720 logical units, letterboxed to the window. Map viewport: x=16, y=48, w=768, h=576;
tile size 32; 24 x 18 visible cells. Sidebar: x=800, y=48, w=464, h=576. Log: x=16, y=640, w=1248, h=64. Header occupies
y=0..47. Leave the unused margin clear. This is not DOS-pixel parity.

Camera origin is floor(hero - visible_cells/2), clamped independently to [0, max(0, map_size-visible_cells)]. Maps
smaller than the viewport begin top-left; remaining space is background and not clickable. Convert mouse coordinates
through the inverse letterbox transform; reject margin clicks rather than selecting a cell. On resize, rendering changes
but the visible-cell contract and simulation do not. Test the default and a 960 x 540 window; improve readability before
shrinking text.

Battle: six by five square cells, 96 units each, top-left (112,96), with active-unit and legal-action information in the
sidebar. Fixed keyboard navigation plus Enter confirm/Escape cancel; adventure arrows/numpad for movement, a visible
action menu for interactions. Do not use real-time key repeat to advance turns behind a modal.

Rebindable controls, fully animated character sets and automatic high-DPI art selection are later improvements, not
prerequisites for testing the game.
