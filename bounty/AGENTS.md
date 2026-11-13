# Bounty

A tribute to King's Bounty (1990, DOS). The repository's `AGENTS.md` applies first; this file adds what is true for
Bounty only. The specification is in `docs/spec/`; `README.md` gives the reading order.

## Rules for every change

- Rebuild the 1990 DOS game, not a later King's Bounty or a Heroes of Might and Magic hybrid. No speed-based turn order
  unless the rules register records evidence for it.
- Only `bounty-core` changes game state. The application, the computer players and the tools submit commands to it. No
  macroquad, window, sound or clock in `bounty-core`, `bounty-ai` or `bounty-tools`.
- A refused command changes nothing: no resources, no state revision, no random draw.
- Rules use integers and a fixed order. No floats, and no iteration over a hash map, where the result decides an
  outcome.
- Random numbers for rules, for a computer player's planning and for presentation come from separate generators; a
  computer player never sees the rules' generator.
- `content/meadow/` is this project's own scenario on the reference's troops, for measuring the campaign's rules with
  the expedition pilot `errand-v1`. Its map and numbers are not the original's, and are never quoted as such.
- `prototype/` and `content/demo/` test the engine. Never tune a computer player on them or quote them as the original's
  balance, and never use their numbers where a number of the original is missing.
- A rule taken from documentation or another reimplementation is recorded in `docs/spec/07-RULES-REGISTER.md` with its
  source and status. Never present it as observed in the original.
- The reference rules (`content/reference/`) never change for the sake of better play. A rule change is a switch of the
  variant `content/modern/`, measured before and after (`docs/spec/10-GAMEPLAY-GAPS.md`), and adopted or dropped by its
  numbers and by play.
- A change to rules, data or a computer player comes with a before/after run of the fixed cases (`balance/`) and a
  sentence on what moved and why. The old baseline stays in git; a missing baseline is not a pass.
- A change in behaviour updates the requirement, its data and its test together (`docs/spec/`, IDs like `G-CMB-005`,
  `AT-013`).
- Files of the original game, anything extracted from them, original saves and captures never enter the repository or a
  release.
- Report only checks that were run.

## Measuring a change

`make balance` plays the cases of every suite in `balance/` (prototype, reference, the `modern` variant) with every seed
and the routes of the expeditions (the demo's, and the meadow's under the reference rules and the variant), and writes a
report for each under `target/` (`BALANCE_ARGS="--case melee_exchange"` picks one case). Compare two reports with

```sh
cargo run -q -p bounty-tools -- balance compare --baseline <before.json> --candidate <after.json>
```

and add `--exact` for a change that must not move any result. `make balance-check` does that against the recorded
baseline, and so does a test, so CI fails when a battle of the suite ends differently or a route of the expedition costs
something else (`bounty-tools expedition run|compare`; time, gold in and out, losses). Replacing the baseline:
`balance/README.md`. A report with problems (a refused command, an outcome the case does not allow, a run that does not
repeat) is a defect to fix, not a result.

## The application

- All text on screen goes through `t!("key")`, with the key in both `crates/bounty-app/assets/i18n/en-GB.ftl` (British
  spelling) and `uk.ftl`. Never hard-code on-screen text; tests check the keys. A stack is always named with its side
  and in one form ("your Wardens", "ваші Вартові"), and sentences are built so that the name is never declined.
- The screen offers only what `Battle::legal_actions` lists (`control.rs`); it has no rules of its own. On the map it
  asks `Expedition::refusal` before it offers a key, and a command refused all the same is told in the log in the
  player's language (`words::refusal_text`).
- Without flags the game is the expedition: `adventure.rs` holds its state, log and save and reads no key, so tests play
  it; `map.rs` draws the map, `view.rs` the battle, alone or inside the expedition (`input.rs` reads both). It is saved
  after every accepted command and continued at the next start; `--rules` or `--scenario` alone gives one battle
  instead. `--expedition demo|meadow` picks the expedition, `meadow` under `--rules reference` or `modern`.
- `--rules reference` plays under the reference rules (battles in `content/reference/battles.json`), `--rules modern`
  under the variant on the same battles; without it the prototype's. The header always says which rules are in use.
- What the rules do to a stack is told where it is shown: its morale with the stacks that lower it, and how many of it
  fight where the leadership does not reach all (`Battle::morale_lowered_by`, `Battle::fighting`). The screen asks the
  battle; it works nothing out by itself.
- Check a changed screen in both languages by screenshot, not by eye on a window:
  `cargo run -p bounty-app -- --lang uk --scenario flight_barrier --steps 3 --shot shot.png`. `--steps` lets the pilot
  play that many commands first; in the expedition it plays that many commands of a route of the fixture
  (`--route <id>`, `complete-route` unless named; 5 stands in the first battle, 99 at the end). With `--shot` or
  `--steps` the user's files are neither read nor written.
- Ukrainian runs longer than English: text with a fixed place uses `text_fit` or `text_wrapped` from `gamekit`.
- A run without `--shot` reads and writes `settings.txt` and, in an expedition, `save-<expedition>-<rules>.json` with
  its `.previous` in the user's data folder: each expedition under each rules has its own save. The user may be playing:
  stop only your own process, by its ID.

## Saving

- A save is the envelope of `schemas/save-envelope.schema.json`, written by `bounty-core::save`: the expedition and the
  battle being fought, in the canonical encoding, with a checksum, the rules' ID and a hash of the three content files.
  `decode` gives an expedition or one reason (`SaveError`): not a save, another version, damaged, other content, or a
  state the rules cannot reach. Nothing is half loaded.
- A new field of `Expedition` or `Battle` goes into `to_canon`, `from_canon` and `check` together; the round-trip tests
  (`save/tests.rs`, `bounty-ai/tests/save_battles.rs`, `expedition_routes.rs`) fail when one is forgotten. A change to
  what is written is a new `SCHEMA_VERSION`; there is no migration yet, an older save is refused by its version.
- The application (`saves.rs`) writes a save with `gamekit::files::write_keeping_previous` and reads it with
  `read_newest`: one current file and the one before it. A file that cannot be read is never deleted or overwritten.

## Not here yet

- Sound, a menu.
- Most of the campaign. Two small expeditions are whole in the rules and on screen: map, clock and weekly budget,
  recruiting, contract, battles with their reward and clue, surrender and recovery, search, save and continue.
  Dwellings, castles, spells on the map, the original's world: not yet.
