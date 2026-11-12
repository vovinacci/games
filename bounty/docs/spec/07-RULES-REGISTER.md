# 07. Rules and fidelity register

## Baseline

Target: original DOS King's Bounty. **Exact executable/version/hash: not selected.** No reference executable was run for
this draft. This blocks an exact-DOS label, not an implementation using explicitly provisional source-backed rules.

Record the legal source of the reference copy, executable hash, displayed version if any, language, emulator
configuration and relevant data-file hashes during M0 when available. Otherwise record the missing source explicitly;
source support cannot be claimed until pinned. A packaging version from a download site is not necessarily the game's
executable version.

## Small confirmed-documentation anchor

The original manual describes per-stack leadership, a five-day week, a siege-equipment prerequisite, a limited spell
inventory, one hero combat spell per round, and a search-based victory. It permits only one retaliation per troop per
round and forbids shooting beside an enemy. A clue can come from a contracted villain or an artefact; failed searches
cost more time than successful ones. These are **M**, not executable-verified conclusions. [S1]

The combat headers corroborate a square board of six columns by five rows and five army slots. [S4]

The reference project/catalogue describes four classes, four continents, 25 troop types, 14 spells, 17 villains and
eight artefacts. Treat those counts as **R** until the selected DOS edition is audited. [S3, S7]

Do not infer hexes, later-game mana/rage, rival kingdom turns or HoMM-style building queues from the franchise name.

## Source conflicts matter

The OpenBounty spec explicitly records differences from both OpenKB and DOS, including purchase-boundary behaviour.
Copying a modern engine can copy its deviations. [S3] Likewise, a manual's descriptive wording may leave an equality
case ambiguous. Only a recorded decision or an executable test should settle such a boundary in Classic.

## Research queue

Each implemented row needs a source-backed decision and a test. Independent DOS observation is required for a
verified-parity claim, not for every provisional release. "Observed" means reproduced against the selected original;
"adopted" means a declared rule choice. Unimplemented subsystems may remain deferred.

| ID | Question | Verification experiment / record | Gate |
|---|---|---|---|
| U-01 | Which exact DOS edition is authoritative? | Record executable/data hashes, version, language and emulator settings. | M0 |
| U-02 | Starting values and progression for every class/difficulty? | Capture start states and each promotion; compare tables and exact thresholds. | M3 |
| U-03 | Travel cost, terrain penalties and boundary ordering? | Walk controlled paths at day/week edges; record before/after state and enemy updates. | M3 |
| U-04 | Sailing/flight access, landing, expiry and region transitions? | Test valid and invalid terrain, lost eligibility, coast transitions and return routes. | M3 |
| U-05 | Weekly income, upkeep, recruitment refresh and insolvency? | Test exact funds, one below, zero, multiple boundaries and temporary modifiers. | M3 |
| U-06 | Leadership equality, injury interactions and loss of control? | Test below/at/above threshold, growth during battle and modifier expiry. | M1b implemented subset |
| U-07 | Permitted army operations and garrison behaviour? | Test same-type recruitment, full army, splitting/merging, dismissal and transfers. | M3 |
| U-08 | Battle turn order, wait queue and phase resets? | Compare slot order with differing movement rates, waits and deaths; do not assume speed initiative. | M1b implemented subset |
| U-09 | Movement, diagonal obstruction, flight and siege layout? | Small obstacle cases and every boundary cell; record costs and reachability. | M1b implemented subset |
| U-10 | Damage formula, skill comparison, rolls and rounding? | Controlled attack pairs and sufficient samples; separate exact arithmetic from distribution estimates. | M1b implemented subset |
| U-11 | Retaliation count source and injury persistence? | Compare first/second attack, partial wounds, death, end of activation and end of battle. | M1b implemented subset |
| U-12 | Ammunition, adjacent shooters, range and special abilities? | Isolate each ability; test interactions with attack, death and control changes. | Implemented scope / parity claim |
| U-13 | Spell eligibility, exact effects, limits and expiry? | One fixture per spell plus invalid-target, cancellation, stacking and boundary cases. | Implemented scope / parity claim |
| U-14 | Contracts, expiry/rotation, escape, relocation and clue rewards? | Defeat the same villain with/without the correct contract; inspect persistent state. | M3 |
| U-15 | Defeat and surrender recovery? | Record all changed and retained fields; compare repeated defeats and depleted funds. | M3 |
| U-16 | Objective placement, puzzle mapping, search cost and final-day precedence? | Known objective fixture, wrong tile, early solution and deadline boundary. | M3 |
| U-17 | Exact maps, random placement pools, counts and reachability? | Compare multiple starts; isolate fixed layout from variable placements. | Implemented scope / parity claim |
| U-18 | Artefact stacking, morale and score formulas? | Controlled before/after records and modifier combinations. | Implemented scope / parity claim |
| U-19 | Enemy battle and wandering policies? | Repeat legal choice situations; record tie-breaks and information available. | Reference AI label |
| U-20 | All content statistics and identifiers? | Populate a provenance-backed catalogue; validate each imported field and ability. | Implemented scope / parity claim |

## Evidence-record template

```text
Evidence ID:
Rule IDs:
Reference build/data hashes:
Initial state or save identification:
Input sequence:
Observed outcome:
Manual statement, if relevant:
Reimplementation lead, if relevant:
Disagreement / uncertainty:
Chosen remake behaviour:
Status: deferred | reference_derived | observed_dos | adopted_deviation
Test fixture and expected result:
Reviewer/date:
```

Store only evidence you have permission to distribute. Private reference saves and captures may remain outside the
public repository; publish a concise reproducible description where appropriate.

## Numeric catalogue policy

Every original numeric value needs a record with field path, value, units, source ID, evidence ID and confidence/status.
Missing values are not zero and are not prototype defaults. The catalogue validator should distinguish unknown from
intentionally absent.

Before a complete campaign profile is release-ready, fill its starting statistics, troop properties/abilities, spell
effects, timing, prices, upkeep, morale, encounters, progress thresholds, artefact modifiers, scoring and maps. This
pack intentionally does not pretend that those historical tables have already been reconstructed.

## Deviation policy

Preserve meaningful rules when known; document intentional fixes. Compatibility does not require reproducing a crash,
unsafe file handling or unreadable controls. A gameplay-changing fix belongs to a named rule revision or an explicit
experimental profile. Explain the difference in release notes.

## Practical research method and stop rule

### Evidence states

Use `observed_dos`, `reference_derived`, `adopted_deviation`, or `deferred`. A pinned OpenKB/OpenBounty implementation
is a permissible provisional semantic reference. Read its documentation and corresponding code; record path/function,
revision and known discrepancies. Two descendant projects are not independent confirmation. Copying source or assets
still requires their applicable licenses.

U-08 concerns activation order, not an instruction to add a speed initiative system. OpenBounty explicitly documents
player-first, slot-order activation [S11]. Test the selected DOS edition if exact ordering is important; do not infer
later HoMM mechanics from the word "speed".

### Investigation sequence

1. Inspect documented formats and source-derived logic first [S8, S9]. Separate extractable tables/maps from executable
   algorithms such as damage or AI.
2. Make a small before/after save-file or memory comparison around one action. Search a bounded range for the known
   visible value; test a second unrelated value and a second initial state before accepting a candidate location.
3. When necessary, use a debugger-enabled DOSBox-X build [S10]. Start it from a terminal on Linux/macOS, use its own
   HELP to verify commands, inspect a known segment:offset with D and dump a bounded region with MEMDUMPBIN. Record the
   executable hash, load segment, encoding, width and signedness. Addresses are edition/run-specific; do not invent a
   universal memory offset.
4. Set an appropriate breakpoint/watchpoint supported by that exact build and inspect the transition, not a screenshot
   approximation. Memory-watch syntax varies; this pack does not pretend a protected-mode breakpoint is a valid
   real-mode KB watch command. Use controlled dumps when a watchpoint is unavailable.
5. Store a concise input/observed-output fixture. Private dumps/saves stay local. Automate repeated inputs and dump
   comparison after the first case is reliable; do not promise wall-clock keyboard scripts are intrinsically
   deterministic.

Tools required initially: source reader, byte/hex viewer, a small local dump-diff script if needed, and the selected
emulator's debugger. A reusable debugger bridge or automated reverse-engineering service is not a project prerequisite.

### Proposed effort cap

Per disputed rule: one source pass and one controlled experiment, capped at four developer-hours unless the maintainer
explicitly chooses to investigate further. This is a proposed project budget, not an estimate for work performed here.
At the cap, adopt the best source-backed behaviour with a visible provisional/deviation label, defer the affected
optional feature, or explicitly prioritise a true blocker. Do not let a low-impact quirk indefinitely block a useful
hobby release.

Prioritise: damage/skill and injury, activation/retaliation, morale/control, affordability/weekly finance, then the
final objective and recovery. Source-backed starting decisions are sufficient to code tests; exact historical claims
require stronger evidence.

No DOS executable, memory experiment or binary parser has been run for v0.3.
