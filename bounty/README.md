# Bounty

A tribute to King's Bounty (1990, DOS): explore, recruit, fight, find the sceptre before time runs out. Rust; the
application will use macroquad. Made for nostalgia, for the fun of programming it, and to work on computer opponents and
game balance.

Nothing to play yet. What exists is the specification and the first crates.

## Layout

| Path | Contents |
|---|---|
| `crates/bounty-core` | rules and state; no graphics, sound or clock |
| `crates/bounty-ai` | computer players: they see what a player sees and answer with a command |
| `crates/bounty-tools` | developer tools: fixed-seed battle runs and their comparison (`make balance`) |
| `docs/spec/` | the specification, see below |
| `prototype/` | synthetic battle rules, scenarios and expected results for testing the engine |
| `content/demo/` | a small synthetic expedition used as an integration test |
| `balance/` | the fixed set of cases and seeds for before/after comparisons, and recorded baselines |
| `schemas/` | JSON schemas of the map and save file |

## Specification

Version 0.3 of 12 November 2026, with the changes in `CHANGELOG.md`. Read in this order:

1. `docs/spec/01-VISION-AND-SCOPE.md` and `06-ROADMAP.md`: scope and sequence.
2. `docs/spec/02-GAME-SPEC.md` and `03-TECHNICAL-DESIGN.md`: behaviour and architecture.
3. `docs/spec/CONTENT-AND-ASSETS.md`: importing, public and private content, presentation.
4. `docs/spec/07-RULES-REGISTER.md`: what is known about the original's rules and how the rest is settled.
5. `docs/spec/09-BALANCE-REGRESSION.md`: measuring the effect of a rules change.
6. `docs/spec/05-ACCEPTANCE-TESTS.md`: the tests each milestone needs.

`docs/spec/REVIEW-RESOLUTION.md` records what a review changed between versions 0.2 and 0.3.

## Two kinds of content

`prototype/` and `content/demo/` are invented for testing commands, arithmetic and saving. They are not the original
game's data and say nothing about its balance.

The original's maps, texts and numbers are not in this repository. The plan is an optional importer that reads them from
the player's own copy of the DOS game; until it exists, rules taken from documentation or other reimplementations are
labelled as such in the rules register.
