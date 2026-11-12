# Regression suites and baselines

A suite lists battles and eight seeds for each, played by the pilot `approach-v1` on both sides. All paths in a suite
file are relative to that file. The battles test the engine; they are not a calibration of King's Bounty balance.

| Suite | Rules | Battles | Prices for valuing losses | Baseline |
|---|---|---|---|---|
| `suite.json` | prototype (`prototype/`) | 4 x 8 | `metrics.json` | `baselines/prototype-0001.json` |
| `reference-suite.json` | reference (`content/reference/`) | 5 x 8 | `reference-metrics.json` | `baselines/reference-0001.json` |
| `modern-suite.json` | the variant (`content/modern/`), on the reference's battles | 5 x 8 | `reference-metrics.json` | `baselines/modern-0001.json` |

The prototype's numbers are never set beside the others': other rules, other troops. The reference's and the variant's
are, on purpose and by hand, in `docs/spec/10-GAMEPLAY-GAPS.md`; the tools hold each to its own baseline.

A test (`each_suite_gives_its_recorded_baseline`) and `make balance-check` compare the code as it stands with the
baselines, battle by battle, down to the final position. So:

- a change that is not meant to move results (a refactor, a new feature beside the old ones) must leave both green;
- a change that is meant to move results fails them, and that is the point where the change gets looked at.

## The recorded baselines

`baselines/prototype-0001.json`: the prototype suite at revision `84956d1`, accepted by the maintainer on 12 November
2026.

| Case | Player wins | Enemy wins | Round limit | Creatures lost, player / enemy |
|---|---|---|---|---|
| `melee_exchange` | 7 | 1 | 0 | 55 / 78 |
| `blocked_shooter` | 7 | 1 | 0 | 77 / 62 |
| `flight_barrier` | 0 | 8 | 0 | 64 / 30 |
| `unreachable_melee` | 0 | 0 | 8, as the case expects | 0 / 0 |

`baselines/reference-0001.json`: the reference suite on top of revision `ba1a341`, with the case `loss_of_control_deep`
added at the maintainer's word on 12 November 2026.

| Case | What it is | Player wins | Enemy wins | Creatures lost, player / enemy |
|---|---|---|---|---|
| `even_fight` | equal armies of Sprites, Wolves and Archers | 6 | 2 | 448 / 500 |
| `morale_sprites_wolves` | equal armies; the player's Sprites have Low morale, the enemy's none | 3 | 5 | 542 / 482 |
| `loss_of_control` | the player's Wolves a little over the leadership; back under control after first losses | 7 | 1 | 131 / 132 |
| `loss_of_control_deep` | the player's Wolves at twice the leadership; out of control most of the battle | 0 | 8 | 608 / 403 |
| `overmatch` | a fight the player must win | 8 | 0 | 0 / 80 |

Eight seeds are few: over 500 seeds `loss_of_control` is won 69 times in 100, not 7 in 8. The baseline pins the engine
down; it is not an estimate of how hard a battle is.

`baselines/modern-0001.json`: the same battles under `modern-v0.1` (the leadership caps the fighters; morale by the
army-view rule), on top of revision `e734935`, as the maintainer set the variant on 13 November 2026.

| Case | Player wins | Enemy wins | Creatures lost, player / enemy | Against the reference |
|---|---|---|---|---|
| `even_fight` | 3 | 5 | 501 / 491 | fewer wins: by the army-view rule the player's Wolves are at Low beside Sprites |
| `morale_sprites_wolves` | 7 | 1 | 160 / 556 | the ten Wolves are halved, not the sixty Sprites |
| `loss_of_control` | 8 | 0 | 98 / 144 | no mutiny; all but one of the Wolves fight |
| `loss_of_control_deep` | 6 | 2 | 489 / 499 | no mutiny; half the Wolves fight |
| `overmatch` | 8 | 0 | 0 / 80 | the same |

## Expedition routes

Battles alone do not show what a rule costs over a whole game. `bounty-tools expedition run` plays the routes an
expedition file carries (its `tests`), the pilot fighting the battles, and reports for each route its time, the gold
that came in (weekly income, rewards) and went out (upkeep, recruits, recovery), the creatures lost and how it ended. A
route whose gold does not add up, that is refused a command or that does not repeat is a problem, not a row.

`baselines/expedition-demo-0001.json`: the four routes of `content/demo/` under the prototype rules. A test
(`the_demo_expedition_gives_its_recorded_baseline`) and `make balance-check` hold the code to it, field by field.

| Route | Ends | Ticks | Gold in: income + rewards | Gold out: upkeep + recruits + recovery | Purse, 100 at start | Creatures lost |
|---|---|---|---|---|---|---|
| `complete-route` | victory | 15 | 0 + 90 | 0 + 40 + 0 | 150 | none |
| `surrender-first-encounter` | goes on | 7 | 0 + 0 | 0 + 40 + 30 | 30 | 12 Wardens, the army given up |
| `timeout-at-town` | timeout | 24 | 20 + 0 | 10 + 0 + 0 | 110 | none |
| `success-on-deadline` | victory | 24 | 20 + 90 | 12 + 40 + 0 | 158 | none |

These are the fixture's own numbers, synthetic like its rules: they pin the engine down and say nothing about the
original's economy. The expedition on real troops gets its own baseline when it exists.

## Replacing a baseline

Only for a change that is meant to move results, in the same pull request as that change:

1. `make balance`, then compare with the old baseline (`make balance-check` prints every battle that moved).
2. Read the differences. Each one is either what the change was for, or a surprise to explain before going on.
3. Save the new report under `baselines/` with the next number (`prototype-0002.json`, `modern-0002.json`), point
   `baseline_path` in the suite file at it (for an expedition: `BALANCE_EXPEDITIONS` in the `Makefile` and the test),
   and keep the old file: the history of baselines stays readable without git.
4. Say in the pull request description what moved and why, with the numbers. That text becomes the commit message, so
   the reason travels with the baseline.

Never replace a baseline to make a failing check pass without having read why it fails. A report that lists problems (a
refused command, an outcome the case does not allow, a run that does not repeat) cannot become a baseline.
