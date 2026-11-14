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
| U-05 | Weekly income, upkeep, recruitment refresh and insolvency? Reference lead, implemented for `content/meadow/`: upkeep is a tenth of a creature's recruiting cost a week, rounded down, taken after the income; a purse too small is emptied and every troop stays (OpenBounty spec 13.4, REQ-264). Not observed in the original. | Test exact funds, one below, zero, multiple boundaries and temporary modifiers. | M3 |
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

## Reference-derived rules in use: `reference-v0.1`

The first rules and numbers of the original game in this repository. All of them are `reference_derived`: read from the
OpenBounty project [S2] at revision `aace551ca625e0c723adff0cb13b22ca2f806dee` (MIT), restated here and typed into
`content/reference/rules.json`. No code was copied. None has been observed in the DOS game by this project, and
OpenBounty describes itself as descended from OpenKB, so the two are one line of evidence, not two.

Files read: `assets/kings-bounty/game.json` (troop table, morale chart), `docs/OPENBOUNTY-SPEC.md` sections 13, 14 and
25, `engine/combat.c`.

### Troops

All twenty-five. A troop is never fielded without an ability it has: a rules file that names one the rules lack is
refused. The first three in the repository were Sprites, Wolves and Archers, chosen to cover the three roles and to make
Low morale reachable (groups A, B and C never give Low among themselves; it needs a troop of group D or E).

| Troop | Role | Skill | Health | Movement | Melee | Shot | Shots | Flies | Abilities | Group | Cost |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Peasants | melee | 1 | 1 | 1 | 1-1 |  | 0 | no |  | A | 10 |
| Sprites | flying | 1 | 1 | 1 | 1-2 |  | 0 | yes |  | C | 15 |
| Militia | melee | 2 | 2 | 2 | 1-2 |  | 0 | no |  | A | 50 |
| Wolves | melee | 2 | 3 | 3 | 1-3 |  | 0 | no |  | D | 40 |
| Skeletons | melee | 2 | 3 | 2 | 1-2 |  | 0 | no | undead | E | 40 |
| Zombies | melee | 2 | 5 | 1 | 2-2 |  | 0 | no | undead | E | 50 |
| Gnomes | melee | 2 | 5 | 1 | 1-3 |  | 0 | no |  | C | 60 |
| Orcs | ranged | 2 | 5 | 2 | 2-3 | 1-2 | 10 | no |  | D | 75 |
| Archers | ranged | 2 | 10 | 2 | 1-2 | 1-3 | 12 | no |  | B | 250 |
| Elves | ranged | 3 | 10 | 3 | 1-2 | 2-4 | 24 | no |  | C | 200 |
| Pikemen | melee | 3 | 10 | 2 | 2-4 |  | 0 | no |  | B | 300 |
| Nomads | melee | 3 | 15 | 2 | 2-4 |  | 0 | no |  | C | 300 |
| Dwarves | melee | 3 | 20 | 1 | 2-4 |  | 0 | no |  | C | 350 |
| Ghosts | melee | 4 | 10 | 3 | 3-4 |  | 0 | no | absorb, undead | E | 400 |
| Knights | melee | 5 | 35 | 1 | 6-10 |  | 0 | no |  | B | 1000 |
| Ogres | melee | 4 | 40 | 1 | 3-5 |  | 0 | no |  | D | 750 |
| Barbarians | melee | 4 | 40 | 3 | 1-6 |  | 0 | no |  | C | 750 |
| Trolls | melee | 4 | 50 | 1 | 2-5 |  | 0 | no | regeneration | D | 1000 |
| Cavalry | melee | 4 | 20 | 4 | 3-5 |  | 0 | no |  | B | 800 |
| Druids | ranged | 5 | 25 | 2 | 2-3 | 10 | 3 | no | magic shot | C | 700 |
| Archmages | flying | 5 | 25 | 1 | 2-3 | 25 | 2 | yes | magic shot | C | 1200 |
| Vampires | flying | 5 | 30 | 1 | 3-6 |  | 0 | yes | leech, undead | E | 1500 |
| Giants | ranged | 5 | 60 | 3 | 10-20 | 5-10 | 6 | no |  | C | 2000 |
| Demons | flying | 6 | 50 | 1 | 5-7 |  | 0 | yes | scythe | E | 3000 |
| Dragons | flying | 6 | 200 | 1 | 25-50 |  | 0 | yes | immune | D | 5000 |

A flying troop that also shoots is listed as flying. The reference writes a magic shot as three numbers of which it
reads the first; here it is that one number.

The troop table, and the table of the heroes' classes below, are exceptions to the rule that the original's numbers stay
out of the repository. The maintainer allowed three troops first, so that the reference rules could be tested in CI; on
14 November 2026 the whole troop table, and on the same day the classes and their ranks: without them neither the rules,
nor a troop's ability, nor a hero's promotion can be tested or measured on what they are for. The exception covers what
a battle and a recruiter need of a troop (the columns above; dwelling, population and growth when the campaign reads
them) and what the king gives a hero. Maps, texts, pictures and the rest of the catalogue stay out.

### The hero's classes and ranks

Four classes of four ranks each (U-02), read from the reference's `game.json` and its text, section 8. Every number of a
rank but the villains it asks for is what the rank adds: a hero has the leadership, commission, spells and spell power
of all the ranks reached, together with whatever was gained elsewhere. The king gives a rank at an audience, once the
hero has caught the villains it asks for; with enough of them, more than one rank at once.

**Knight**: sets out with 7500 gold and 20 Militia, 2 Archers.

| Rank | Villains | Leadership | Spells | Spell power | Commission | Knows magic | Instant army |
|---|---|---|---|---|---|---|---|
| Knight | 0 | 100 | 2 | 1 | 1000 | no | Peasants |
| General | 2 | 100 | 3 | 1 | 1000 | no | Militia |
| Marshal | 8 | 300 | 4 | 1 | 2000 | no | Archers |
| Lord | 14 | 500 | 5 | 2 | 4000 | no | Knights |

**Paladin**: sets out with 10000 gold and 20 Peasants, 20 Militia.

| Rank | Villains | Leadership | Spells | Spell power | Commission | Knows magic | Instant army |
|---|---|---|---|---|---|---|---|
| Paladin | 0 | 80 | 3 | 1 | 1000 | no | Peasants |
| Crusader | 2 | 80 | 3 | 1 | 1000 | no | Militia |
| Avenger | 7 | 240 | 6 | 2 | 2000 | no | Archers |
| Champion | 13 | 400 | 5 | 2 | 4000 | no | Cavalry |

**Sorceress**: sets out with 10000 gold and 30 Peasants, 10 Sprites.

| Rank | Villains | Leadership | Spells | Spell power | Commission | Knows magic | Instant army |
|---|---|---|---|---|---|---|---|
| Sorceress | 0 | 60 | 5 | 2 | 3000 | yes | Sprites |
| Magician | 3 | 60 | 8 | 3 | 1000 | no | Gnomes |
| Mage | 6 | 180 | 10 | 5 | 1000 | no | Elves |
| Archmage | 12 | 300 | 12 | 5 | 1000 | no | Druids |

**Barbarian**: sets out with 7500 gold and 20 Wolves.

| Rank | Villains | Leadership | Spells | Spell power | Commission | Knows magic | Instant army |
|---|---|---|---|---|---|---|---|
| Barbarian | 0 | 100 | 2 | 0 | 2000 | no | Peasants |
| Chieftain | 1 | 100 | 2 | 1 | 2000 | no | Wolves |
| Warlord | 5 | 300 | 3 | 1 | 2000 | no | Orcs |
| Overlord | 10 | 500 | 3 | 1 | 2000 | no | Ogres |

"Knows magic" is set by one class's first rank only; the others learn it at a place on the map. "Instant army" names the
troop a spell of that name raises at the rank. Neither is read by anything yet: they wait for the spells.

How it is carried out here, in an expedition whose file names a king (his castle on the map, the hero's class):

- The hero sets out with what the first rank gives. The file must say the same leadership and weekly income, so that
  there is one answer to what a Knight starts with.
- A band beaten while its contract is held is a villain caught; beaten without the contract it is only beaten (gap C4 of
  `10-GAMEPLAY-GAPS.md` is the reference's rule, kept).
- In the king's castle the hero asks for his rank. Every rank the villains caught have earned is given at once, each
  adding its leadership and commission; the commission is the week's income from then on, and recruiting and battles use
  the new leadership. Asked elsewhere, or with too few villains, or at the highest rank, the command is refused and
  nothing changes.
- An audience takes no time. Status: `adopted_deviation`; the reference does not say what an audience costs, and nothing
  on the way to the castle is free.
- Spell limit and spell power of a rank are not kept in the hero's state yet. They come with the spells.

An expedition without a king is as it was: leadership and income stay what it starts with, and its saved state is
written as before. The hero's standing (rank, leadership, commission, villains caught) is written only where there is a
king, and a save whose standing is not what its rank and villains make it is refused.

No expedition in the repository has a king yet, and the game has no key for the audience: both come next.

### Rules, and how they differ from the prototype

| ID | Rule as the reference has it | Prototype | Doubt |
|---|---|---|---|
| U-08 | The player's stacks act in slot order, then the enemy's. There is no speed order. | The same order. | None known. |
| U-08 | Flags and allowances (struck back, movement, flights) are renewed for every stack before each side's turn, so a stack can strike back once in its own side's turn and once in the other's. | Renewed once a round. | The reference's own code; not checked against DOS. |
| U-09 | A step goes to any of the eight neighbours; a diagonal step is not stopped by what stands beside it. Stepping onto an enemy is the melee attack. | Diagonal needs both cells beside it free; attack is a separate action. | None known. |
| U-09 | A flyer has two flights in its turn. A flight costs no movement; the second ends the turn. | One flight, costing a movement point. | None known. |
| U-10 | Damage: roll x creatures x (attacker's skill + 5 - target's skill) / 10, each step in whole numbers, rounding down. | Roll x creatures. | The reference calls its routine a transcription of the original's. |
| U-10 | The count used is the stack's count before the exchange, for the answer as well: creatures killed by a blow still strike back. | The answer uses the survivors. | As above. |
| U-10 | Morale then scales the damage of a stack that is on the hero's side and under control: Low halves, High adds half. The enemy has no hero, so no morale. | No morale. | See below. |
| U-11 | One answer per stack between renewals; none to a shot. Injury carries over inside a battle, as a health pool. | The same pool; one answer a round. | None known. |
| U-12 | A stack cannot shoot with an enemy in one of the eight cells around it. A shot uses one of the troop's shots whatever it does. | The same. | None known. |
| U-12 | Regeneration (Trolls): when a round begins, a stack that regenerates has no injury, on either side. Not when the enemy's part of the round begins. | No abilities. | None known. |
| U-12 | Absorb (Ghosts): after a blow of the stack, struck back included, the creatures the blow killed join it. There is no limit, so the stack can outgrow the hero's leadership and go out of control. | No abilities. | Where the blow kills the whole target the reference counts the target's number at the start of the turn, not the number the blow killed; here it is what the blow killed, as for U-10. |
| U-12 | Leech (Vampires): after a blow of the stack, one creature returns for every whole health of its own in what the blow took, which is counted as the full health of the creatures killed plus the injury now on the target. Above the number the stack began the battle with, it has that number and no injury; at or below it, the injury stays. | No abilities. | That the target's earlier injury counts as taken by this blow reads like an accident of the reference's sum; kept, as it is what the reference does. The same doubt about a target killed whole as for Absorb. |
| U-12 | Undead (Skeletons, Zombies, Ghosts, Vampires): a mark that one spell reads. Nothing else in a battle does; morale treats the undead like any troop. | No abilities. | None known. |
| U-12 | Scythe (Demons): before every blow of the stack, struck back included, a number from 1 to 100 is rolled; above 89 the scythe takes half the target as it stands, rounded up, at full health each, on top of the blow. It is rolled for before the blow's damage and added after morale has scaled the blow. | No abilities. | The reference's text calls it 11 in 100, which the roll gives; nothing says what the original rolls. |
| U-12 | Magic shot (Druids, Archmages): the shot's number is fixed and nothing is rolled; skill and morale scale it like any shot. Against an immune troop it does nothing at all, and the shot is spent. | No abilities. | None known. |
| U-12 | Immune (Dragons): magic shots, and the spells that are not here yet, do nothing to the troop. Arrows and blows do. | No abilities. | None known. |
| U-06 | A stack is out of control when health of one x count is more than the hero's leadership (equal is still in control). It keeps its turns and counts as an enemy of its own side, until losses bring it back under; checked before every stack acts. | No leadership in battle. | Who moves it during the player's turn is the reference's choice (its computer player). |
| | Random numbers: a 64-bit generator, state x 25214903917 + 11, the top 32 bits taken modulo the size of the range; a range of one number draws nothing. | A 32-bit generator with rejection. | The modulo favours low numbers slightly; kept as the reference has it. |

### Adopted where the reference is silent or impractical

- **Waiting.** In the reference a stack that waits is passed over and comes up again after the other stacks of its side;
  nothing there says what a second wait does. Here, as in the prototype, a stack may wait once before it has moved, acts
  after the others of its side, and a second wait ends its turn. Status: `adopted_deviation`.
- **Seeding.** The reference derives a battle's first random state from the world's seed and the encounter's identity.
  There is no world yet; a battle here starts from the state the scenario or the suite gives it.

### Implemented so far

Everything in the table above: turn order by sides with the renewal of flags, the step rule, flight, shots, damage
scaled by skill with the count from before the exchange, morale by the ported rule, loss and return of control, and the
reference generator; and every ability of a troop.

A stack that grew by absorbing keeps its number after the battle: the army has what the battle left. What a leech began
with is part of a saved battle, for the troops that leech and no others, so that every other battle is encoded as it
was.

How loss of control is carried out here:

- A scenario gives the leadership of the player's hero. Without it the player's side has no hero in that battle: no
  morale and no stack out of control. The enemy never has a hero.
- A stack out of control and any other stack fight each other, whatever their sides: it may be struck and shot by its
  own side, it may strike and shoot its own side, and a stack of its side standing beside it stops a shooter as an enemy
  would. Its creatures still count for its side when the battle is decided.
- It has no morale.
- Control comes back, and only comes back: a stack that starts within the leadership stays in control, as in the
  reference, which sets the mark when the battle is set up and only ever clears it.
- The rules do not say who commands it. The reference hands its turn to its computer player; here the pilot plays it, in
  the runs and on screen, where the stack is marked and the sidebar says that it moves by itself.

### The two morale rules

The reference documents two ways to find a stack's morale and says which content uses which:

- **Ported**, for the King's Bounty content, described there as the original's behaviour "including the documented bug":
  every living stack of the side, the stack itself included, is looked up in the chart as (other, self); the ranks are
  ordered Normal, Low, High, and the lowest found wins. So a mixed army falls to Normal as soon as one pairing is
  Normal, and reaches Low only when no pairing is Normal and one is Low.
- **Army view**, for its other content: a stack alone is High; every other stack is looked up as (self, other); any Low
  gives Low, all High gives High, else Normal. This is what the army screen shows the player.

`reference-v0.1` uses the ported rule. The two disagree, for instance, on an army of Sprites and Wolves: ported makes
the Sprites Low and the Wolves Normal; army view makes the Wolves Low and the Sprites Normal. That army is the morale
case of the reference suite. Which of the two the DOS game really does is open (U-18) and worth one experiment, since it
decides what the player is told against what the dice do.

### Not in this subset

Refused on load, not approximated: hero spells in battle, artefact powers, siege layout, the random field obstacles of
an encounter, spoils. Each comes with the first troop or encounter that needs it.

## A variant, not a source: `modern-v0.1`

`content/modern/rules.json` holds the reference rules with four switches set otherwise, to try the rule changes proposed
in `10-GAMEPLAY-GAPS.md`. Nothing in it is evidence about the original, and nothing it changes is ever written back into
`reference-v0.1`. The loader lets only this ruleset set a switch; the reference file with a variant's value is refused.
The variant plays the reference's battles and keeps its own baselines.
