# Specification changelog

## In this repository, after 0.3

- Named the game Bounty: folder `bounty`, crates `bounty-*`, encoding id `bounty-cjson-v1`. Requirement, test and issue
  IDs are unchanged.
- English and Ukrainian from the first screen (D-SCP-007).
- Added the boundary with the shared `gamekit` crate (T-ARC-004).
- Dropped the pack's Python checkers; the Rust tests take over their checks. Cyrillic in `canonical_vectors.json` is
  written as escapes; the values are the same.
- Replaced the pack's `README.md` and `AGENTS.md` with ones for this repository.

## 0.3 - 12 November 2026

- Incorporates the supplied review; gap 1 is treated as previously resolved.
- Adds local DOS-data import as the proposed content architecture, with an explicit
  unsupported-edition failure and no automatic downloader or bundled original data.
- Allows pinned reimplementations as provisional semantic references, records
  shared ancestry, and bounds original-executable investigation by risk and effort.
- Defines procedural presentation and audio; imported sprites are optional, not
  a prerequisite for a playable public development build.
- Replaces the early benchmark framework with a small runner, frozen seeds, raw
  results and a reviewed diff. No M0 synthetic comparator project, approval CLI,
  mandatory statistical envelopes, or multi-mode reporting framework.
- Retains prototype JSON unchanged as engineering fixtures only. A reference-rule
  subset is required before KB difficulty, fun, or AI tuning claims.
- Specifies map/expedition data, viewport, save encoding, canonical digest and
  repository-relative integration. Adds schemas and test vectors.
- Defers remapping UI, selectable filters, multiple save slots, rotating autosave,
  general save migrations and advanced AI search. Preserves safe current-version
  saving, corrupt-file recovery and a single previous save backup.
- Removes any implication that runtime import automatically establishes legal
  clearance, executable parity, or rights to redistribute extracted content.

No original-game rules or numerical balance were independently verified in this
revision. Prototype rules remain `prototype-v0.1`. Balance contract becomes
schema 2; v0.2 baseline reports must not be reinterpreted under that schema.
