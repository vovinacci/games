# Delivery sequence after review

Milestones describe outcomes, not time estimates. No general-purpose infrastructure is a prerequisite for a small
playable battle.

## M0 - Integrate and choose the first reference subset

Use the parent workspace. Keep four package boundaries but introduce only the functions needed by the first battle.
Confirm the technical game path and public content policy. Select one source edition and a provisional rule reference.
When original data are unavailable, record that fact and proceed with synthetic correctness tests; do not silently
select a different edition.

Exit: the small first issue is unambiguous. No benchmark-management application, full content catalogue, original art
collection or exhaustive DOS research gate.

## M1a - One executable engineering battle

Load the retained fixtures; implement state, atomic commands, melee/damage and retaliation. In that same increment
record fixed-seed results and deterministic reruns. Add movement, ranged attacks, flight and queues with their tests.

Exit: a headless battle completes and the small before/after report works. The result proves implementation correctness
only, not KB balance or fun.

## M1b - First KB-specific battle

Import or locally define three source-backed representative troops for a selected reference: melee, ranged and flight.
Implement the rules exercised by them, including skill, morale, leadership/control and activation semantics. Provisional
source-backed adoption is permitted, with contradictions recorded. Do not add speed-order initiative by analogy to
another game.

Use procedural tokens and the fixed layout. Add one normal fight, one matchup sensitive to skill/morale/control, and one
deliberate overmatch. Freeze an explainable pilot and establish the reference-profile baseline. No aggressive AI tuning
until this is in place. Unsupported special units are rejected, not silently simulated as generic troops.

Exit: a small recognizable KB rules subset can be played and reproduced headlessly; its outcomes and known deviations
are inspectable. No exact-DOS claim without tests.

## M2 - Complete small expedition

Use content/demo/expedition.json as the exact synthetic integration contract. For KB playtesting, author a second small
scenario using the supported imported rule/content subset; do not treat the synthetic demo's economy as Classic tuning.
Recruit, travel, fight, obtain a clue, search, win/fail and save/continue. Record days, gold and troop losses alongside
battle outcomes.

Exit: the full loop works, a current-version save survives restart, and the measured expedition report exposes
casualty/economic consequences. No map editor.

## M3/M4 - Expand systems, data and AI only as exercised

Add remaining movement, location, spell, progression and recovery mechanics with representative fixtures. Implement the
first actual local-data adapter for one selected edition; report supported components and unsupported fields explicitly.
Broaden the reference suite, then tactical AI and held-out cases. Reference research is bounded by the policy in
07-RULES-REGISTER.md, not an endless M4 gate.

Exit: a complete labelled campaign is playable. Known differences are listed; unresolved details cannot be marketed as
verified parity. Import support is claimed only for an edition and assets that have actually been tested.

## M5 - Release the engine/demo, not somebody else's assets

Only the app is packaged publicly through the parent's release machinery. Run the distribution allowlist check, verify
controls and both intended UI languages, exercise current save recovery and supported platforms. Explain that
original-data play requires local user-supplied files. No data downloader.

## First issues (IDs retained where useful)

| ID | Implement | Evidence |
|---|---|---|
| KB-001 | Record target/source choice and integration path. | Known versus provisional rule list. |
| KB-002 | Add the four package boundaries to the existing workspace. | Core/tools run without graphics; no root replacement. |
| KB-003/004 | Fixture loading, state, canonical encoding and atomic errors. | JSON validation; vectors; failure leaves state/RNG unchanged. |
| KB-006 | Melee, health, retaliation. | AT-012/013/014 plus completed encounter. |
| KB-013/014 | Small fixed-seed runner and outcome diff, with KB-006. | Real rows, deterministic rerun and explicit first baseline review. |
| KB-005/007/008/009 | Movement/range/flight/queues, replay, simple pilot. | Applicable rule cases and expanded smoke coverage. |
| KB-016 | Source-backed reference subset and minimal local-data adapter probe. | Three real troop records and rule provenance; no guessed values. |
| KB-010 | Procedural battle UI. | Same commands/state as headless runner. |
| KB-011 | One current-format save plus previous backup. | Mid-battle restoration and corrupt/incompatible-file rejection. |
| KB-012/015 | Demo expedition, then reference-content expedition and report. | Complete route, recovery/timeout paths and cost deltas. |

KB-013 no longer means a standalone comparator framework. The original v0.2 coding brief is withdrawn.

## Explicitly deferred convenience work

| Feature | Safe minimum now | Later trigger |
|---|---|---|
| Key remapping screen | Fixed documented keyboard controls. | Actual accessibility/layout need. |
| Sprite/filter selector | Procedural art; nearest sampling if sprites are imported. | Multiple supported visual styles. |
| Save slot browser/rotation | One save plus one previous valid backup. | More than one campaign becomes a use case. |
| General migrations | Reject unknown schemas; preserve bytes. | A published format has a supported successor. |
| Advanced AI search/telemetry | Simple versioned pilot and a short result diff. | Reference rules and baseline are established. |
| Custom portraits/music | Generic tokens and six simple authored sound cues. | Gameplay is worth expanding presentation for. |

## First coding brief

Integrate KB-002 through KB-004 without replacing the parent workspace. Implement KB-006 and its fixed-seed result
recording as one small next increment. Do not implement the old M0 comparator framework, a UI engine, or a map editor.
Preserve synthetic/reference labels. Report executed checks and unresolved source choices.
