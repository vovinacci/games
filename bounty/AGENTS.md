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
and writes a report for each under `target/` (`BALANCE_ARGS="--case melee_exchange"` picks one case). Compare two
reports with

```sh
cargo run -q -p bounty-tools -- balance compare --baseline <before.json> --candidate <after.json>
```

and add `--exact` for a change that must not move any result. `make balance-check` does that against the recorded
baseline, and so does a test, so CI fails when a battle of the suite ends differently. Replacing the baseline:
`balance/README.md`. A report with problems (a refused command, an outcome the case does not allow, a run that does not
repeat) is a defect to fix, not a result.

## The application

- All text on screen goes through `t!("key")`, with the key in both `crates/bounty-app/assets/i18n/en-GB.ftl` (British
  spelling) and `uk.ftl`. Never hard-code on-screen text; tests check the keys. A stack is always named with its side
  and in one form ("your Wardens", "ваші Вартові"), and sentences are built so that the name is never declined.
- The screen offers only what `Battle::legal_actions` lists (`control.rs`); it has no rules of its own.
- `--rules reference` plays under the reference rules (battles in `content/reference/battles.json`), `--rules modern`
  under the variant on the same battles; without it the prototype's. The header always says which rules are in use.
- What the rules do to a stack is told where it is shown: its morale with the stacks that lower it, and how many of it
  fight where the leadership does not reach all (`Battle::morale_lowered_by`, `Battle::fighting`). The screen asks the
  battle; it works nothing out by itself.
- Check a changed screen in both languages by screenshot, not by eye on a window:
  `cargo run -p bounty-app -- --lang uk --scenario flight_barrier --steps 3 --shot shot.png`. `--steps` lets the pilot
  play that many commands first. With `--shot` the user's settings are neither read nor written.
- Ukrainian runs longer than English: text with a fixed place uses `text_fit` or `text_wrapped` from `gamekit`.
- A run without `--shot` reads and writes `settings.txt` in the user's data folder. The user may be playing: stop only
  your own process, by its ID.

## Not here yet

- Sound, a menu, saving.
- Most of the campaign. `bounty-core::campaign` has the expedition of `content/demo/` whole: map, clock and weekly
  budget, recruiting, contract, battles with their reward and clue, surrender and recovery, search. Saving and the
  adventure screen are to come.
