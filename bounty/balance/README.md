# Regression suite and baselines

A suite lists four battles and eight seeds for each: 32 battles, played by the pilot `approach-v1` on both sides. All
paths in a suite file are relative to that file. The battles test the engine; they are not a calibration of King's
Bounty balance.

| Suite | Rules | Prices for valuing losses | Baseline |
|---|---|---|---|
| `suite.json` | prototype (`prototype/`) | `metrics.json` | `baselines/prototype-0001.json` |
| `reference-suite.json` | reference (`content/reference/`) | `reference-metrics.json` | none yet |

The reference suite has battles of Sprites, Wolves and Archers: equal armies; equal armies in which the player's Sprites
have Low morale and the enemy's have none; the player's Wolves starting out of control; and a fight the player must win.
Its numbers are never set beside the prototype's: other rules, other troops. Until its first numbers have been looked at
it has no baseline, and a run of it is compared with nothing.

## The recorded baseline

`baselines/prototype-0001.json` is the report of the suite at revision `84956d1`, accepted by the maintainer on 12
November 2026 as the first baseline.

| Case | Player wins | Enemy wins | Round limit | Creatures lost, player / enemy |
|---|---|---|---|---|
| `melee_exchange` | 7 | 1 | 0 | 55 / 78 |
| `blocked_shooter` | 7 | 1 | 0 | 77 / 62 |
| `flight_barrier` | 0 | 8 | 0 | 64 / 30 |
| `unreachable_melee` | 0 | 0 | 8, as the case expects | 0 / 0 |

A test (`the_suite_gives_the_recorded_baseline`) and `make balance-check` compare the code as it stands with this file,
battle by battle, down to the final position. So:

- a change that is not meant to move results (a refactor, a new feature beside the old ones) must leave both green;
- a change that is meant to move results fails them, and that is the point where the change gets looked at.

## Replacing the baseline

Only for a change that is meant to move results, in the same pull request as that change:

1. `make balance`, then compare with the old baseline (`make balance-check` prints every battle that moved).
2. Read the differences. Each one is either what the change was for, or a surprise to explain before going on.
3. Save the new report as `baselines/prototype-NNNN.json` with the next number, point `baseline_path` in `suite.json` at
   it, and keep the old file: the history of baselines stays readable without git.
4. Say in the pull request description what moved and why, with the numbers. That text becomes the commit message, so
   the reason travels with the baseline.

Never replace a baseline to make a failing check pass without having read why it fails. A report that lists problems (a
refused command, an outcome the case does not allow, a run that does not repeat) cannot become a baseline.
