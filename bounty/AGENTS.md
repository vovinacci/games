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
- A change to rules, data or a computer player comes with a before/after run of the fixed cases (`balance/`) and a
  sentence on what moved and why. The old baseline stays in git; a missing baseline is not a pass.
- A change in behaviour updates the requirement, its data and its test together (`docs/spec/`, IDs like `G-CMB-005`,
  `AT-013`).
- Files of the original game, anything extracted from them, original saves and captures never enter the repository or a
  release.
- Report only checks that were run.

## Measuring a change

`make balance` plays the cases of `balance/suite.json` with every seed and writes `target/bounty-balance.json`
(`BALANCE_ARGS="--case melee_exchange"` picks one case). Compare two reports with

```sh
cargo run -q -p bounty-tools -- balance compare --baseline <before.json> --candidate <after.json>
```

and add `--exact` for a change that must not move any result. A report with problems (a refused command, an outcome the
case does not allow, a run that does not repeat) is a defect to fix, not a result.

## Not here yet

- `bounty-app`: added when its first code arrives.
- A recorded baseline: all four cases of the suite run, but `balance/baselines/bootstrap.json` says `not_recorded` until
  the maintainer has looked at the first numbers.
