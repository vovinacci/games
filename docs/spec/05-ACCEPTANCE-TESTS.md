# 05. Acceptance tests and traceability

These are required tests, not reports of tests already passed by a game implementation. The supplied JSON fixtures have
only undergone structural/arithmetic checks. **P** denotes a fully specified prototype expectation. **C** requires a
Classic rule resolved in the research register. **S** is a system invariant independent of historical tuning.

## Foundation and commands

| Test | Requirement | Given / when | Expected |
|---|---|---|---|
| AT-001 S | T-ARC-001 | Build and test core/tools on a headless runner. | No graphics context, audio device or window required. |
| AT-002 S | T-CMD-002/003 | Submit an illegal move, stale actor or unaffordable purchase. | Error; byte-equivalent canonical gameplay state and unchanged RNG. |
| AT-003 S | T-RNG-003/004 | Same state and accepted command tape, repeated and on supported targets. | Same checkpoints and terminal state. |
| AT-004 S | T-RNG-002 | Change animation speed, sound and presentation randomness. | Identical gameplay state and random streams. |
| AT-005 S | G-FLOW-003 | Open/close each modal, switch language, resize. | No time, funds, charge, reward or RNG change. |
| AT-006 S | T-CMD-003 | Target stack dies before a queued command is submitted. | Stale/invalid command rejected; replacement occupant is not targeted. |

## Tactical rules

| Test | Requirement | Given / when | Expected |
|---|---|---|---|
| AT-010 P | G-CMB-001/003 | Board 6 by 5; move beyond any edge. | Rejected without mutation. |
| AT-011 P | G-CMB-003 | Attempt diagonal movement through a blocked orthogonal neighbour. | Rejected by prototype corner rule. |
| AT-012 P | G-CMB-005/006 | 10 creatures of 10 HP take 26 damage. | 74 HP; 8 creatures; 6 residual injury. |
| AT-013 P | G-CMB-005/007 | Fixture melee exchange, forced rolls 3 then 2. | Defender: 7 creatures, 0 injury. Attacker: 9 creatures, 4 injury. Defender retaliation used. |
| AT-014 P | G-CMB-007 | A second melee attack strikes that defender in the same round. | No second retaliation. |
| AT-015 P | G-CMB-004 | Shooter is beside an enemy. | Shoot absent from legal actions; attempted shot rejected. |
| AT-016 P | G-CMB-003 | Shooter uses last shot, then begins a later round. | Ammunition remains empty. |
| AT-017 P | G-CMB-003 | Flyer crosses an obstruction to a free cell. | Legal under prototype flight; landing on obstruction/occupancy rejected. |
| AT-018 P | G-CMB-003/007 | A stack waits, then acts in the deferred queue. | Acts once, with the specified budget; second wait becomes pass. |
| AT-019 P | G-CMB-006 | Mend targets a living stack with 6 injury. | Injury becomes 0; creature count does not rise. |
| AT-020 P | G-MAG-001 | Cancel spell selection or select an invalid target. | No charge or per-round spell allowance consumed. |
| AT-021 P | G-MAG-002 | Enemy troop controller requests a hero spell. | Rejected even when a player test pilot has that spell. |
| AT-022 S | G-CMB-010 | Finish battle, skip result animation, reload result. | Campaign write-back and rewards occur once. |
| AT-023 S | G-CMB-011 | Neither army can reach or damage the other. | Harness classifies stalled/budget-exceeded, not normal defeat. |
| AT-024 C | G-ARM-003 | Below/at/above leadership threshold and later forced growth. | Matches the recorded U-06 decision and fixture. |
| AT-025 C | G-CMB-008 | Every special ability plus death/retaliation/control interactions. | Matches individually approved fixtures; no generic fallback. |
| AT-026 C | G-CMB-005/006 | Skill mismatch, zero/minimum damage, injury and health restoration. | Exact approved arithmetic, including rounding sequence. |

## Campaign, economy and progress

| Test | Requirement | Given / when | Expected |
|---|---|---|---|
| AT-030 S | G-WLD-002 | Convert reference coordinates and back at all corners. | Round-trip equality; known landmarks remain correctly placed. |
| AT-031 S | G-WLD-006/007 | Choose a known route near unknown territory. | No hidden-object information leaks; travel stops for encounters. |
| AT-032 S | G-TIM-001/002 | Action crosses two weekly boundaries. | Two settlements, each exactly once and in documented order. |
| AT-033 P | G-ECO-003 | Price 20, funds 20, capacity and stock available. | Buy one; funds 0, stock down one, army up one. |
| AT-034 S | G-ECO-003 | Same purchase lacks stock/capacity. | Every field unchanged, including RNG and ledger. |
| AT-035 C | G-ECO-002 | Income/upkeep/shortage and modifier expiry at boundary. | Matches approved U-05 ordering. |
| AT-036 S | G-ARM-001/005 | Reload hero with artefact and reopen promotion dialogue. | No duplicate multiplier or rank reward. |
| AT-037 S | G-ARM-006 | Transfer between full army and garrison; cancel or fail. | Troop conservation; no loss or duplication. |
| AT-038 C | G-QST-001 | Defeat target with/without appropriate contract. | Capture, reward, relocation and clue state match U-14. |
| AT-039 S | G-QST-002/003 | Reopen puzzle, resize, inspect hidden cells. | Reveal mask unchanged; no hidden text/hover leak. |
| AT-040 S | G-QST-004 | Infer objective before every clue, then perform legal search. | Resolution follows location and rules, not an invented all-clues gate. |
| AT-041 C | G-QST-004 | Search succeeds on final permitted boundary. | Matches approved U-16 terminal-order decision. |
| AT-042 C | G-END-001 | Lose or surrender with unusual transport/economy state. | Complete recovery transaction; no stranded invalid state. |
| AT-043 S | G-WLD-003 | Temporary movement eligibility expires on special terrain. | Defined recovery or legal state; never panic or silent free exploit. |
| AT-044 S | G-END-002 | Reload victory/timeout save. | Same terminal status and summary, no duplicate rewards. |

## Persistence, data and UI

| Test | Requirement | Given / when | Expected |
|---|---|---|---|
| AT-050 S | T-SAV-001/002 | Save during battle after waiting, retaliation and spell use. | Reload preserves the next legal actions, queues and RNG. |
| AT-051 S | T-SAV-003 | Interrupt the current-save and single-backup write path at each phase. | Previous valid save is recoverable; success is not falsely reported. |
| AT-052 S | T-SAV-004 | Current save/unsupported schema/corrupt save. | Current format loads; other formats fail safely; original bytes preserved. Migration only when separately supported. |
| AT-053 S | T-CNT-002 | Missing troop, invalid ability, oversize map or duplicate ID. | Useful validation error before gameplay begins. |
| AT-054 S | T-CNT-003 | Catalogue reordered without semantic changes. | Stable IDs preserve interpretation. |
| AT-055 S | T-REP-001/002 | Replay after changing AI implementation only. | Recorded commands reproduce the same game. |
| AT-056 S | G-UX-004/006 | Screenshot each screen in both languages and small windows. | No clipping, invisible focus or overlapping mandatory information. |
| AT-057 S | G-UX-005 | Complete slice with keyboard, muted sound and instant animation. | Every mandatory action remains usable and understandable. |
| AT-058 S | G-DEV-002 | Assemble public release. | Every shipped asset has a recorded source and license/permission status. |

## AI and balance

| Test | Requirement | Given / when | Expected |
|---|---|---|---|
| AT-060 S | A-POL-001 | Policies play the fixed legal-action suite and random scenarios. | Zero illegal returned commands. |
| AT-061 S | T-OBS-001/003 | Change hidden state and true future RNG while observation is identical. | Same policy output for identical private policy RNG. |
| AT-062 S | T-RNG-005 | Run search in different frame chunk sizes. | Same decision for the same total work budget. |
| AT-063 S | A-DIF-001/002 | Change tactical policy only. | Same initial stats, finances, rule parameters and legal information. |
| AT-064 S | A-BEN-001/002 | Run versioned paired benchmark suite. | Complete metrics including stalls and losses, not wins alone. |
| AT-065 S | A-BEN-004 | Evaluate tuned policy on held-out scenarios. | Results reported separately; regressions remain visible. |

## Small balance runner and revised gates

| Test | Requirement | Check | Expected |
|---|---|---|---|
| AT-070 | B-REG-001 | First complete headless battle. | A stored result and repeat run exist in the same increment. |
| AT-071 | B-REG-002 | Repeat fixed build, fixture, pilot and seed. | Same ordered gameplay output. |
| AT-073 | B-REG-005 | Missing/duplicate run, illegal action or engine error. | Required suite fails; no silent denominator reduction. |
| AT-074 | B-REG-005 | Deliberately unreachable fixture. | Expected diagnostic, never counted as defeat. |
| AT-076 | B-REG-004/005 | No measured baseline. | Explicit unrecorded result, not passing balance comparison. |
| AT-077 | B-REG-003 | Price changes but casualties do not. | Fixed-price metric stays constant; native cost changes. |
| AT-078 | B-REG-006 | Deliberately modify a sensitive rule and revert. | Selected result changes, then returns; no fake baseline. |
| AT-079 | B-REG-004 | Intended gameplay change. | Before/after files and reviewed explanation retained. |
| AT-081 | B-REG-006 | Expedition route. | Days, losses, spending and outcome reported. |

AT-072/075/080/082/083 no longer require a five-mode/state-machine comparator. Their useful concerns (input
compatibility, coverage and meaning of metrics) are ordinary assertions in the runner and reviewer checklist, not
separate M0 projects.

## Review-gap acceptance checks

| Test | Required evidence |
|---|---|
| AT-090 | Parent workspace builds without changing other games' settings; internal packages are not published. |
| AT-091 | All canonical vectors match; duplicate keys, floats and invalid Unicode are rejected. |
| AT-092 | Supplied map validates; malformed rows, unknown terrain, duplicate/out-of-bounds objects fail. |
| AT-093 | Supplied demo route reaches its specified result; the checks are labelled synthetic. |
| AT-094 | Unknown DOS edition fails cleanly; originals remain byte-identical after import. |
| AT-095 | Supported adapter's selected troop fields/map landmarks match its recorded source fixture. |
| AT-096 | Public bundle allowlist excludes original input, extracted caches, private saves and captures. |
| AT-097 | Before KB AI tuning, the reference subset exercises skill, morale, control and actual activation order. |
| AT-098 | One current-version save/backup restores mid-battle; incompatible/corrupt saves preserve source bytes. |

M0: integration/source decisions only; no fake numerical baseline. M1a: core correctness and first result recording.
M1b: reference-rule cases and procedural playable view, with source-derived versus observed status explicit. M2: the
whole expedition and safe save/continue, plus economy report. M3/M4: coverage for implemented content and a documented
fidelity/deviation ledger. M5: supported-platform, public content, language and release checks. General migrations,
remapping UI and rotating save slots are not MVP gates.

Property tests should add random valid states and command sequences, checking occupancy, non-negative resources, stable
IDs, no zero-health living units, determinism and transactional failure. Golden tests catch changed answers; property
tests catch broken invariants. Neither alone proves historical correctness.
