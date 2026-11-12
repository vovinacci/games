# 03. Technical design

All architecture in this document is proposed for this project. It is not a claim about how the DOS game was
implemented.

## Dependency direction

```text
bounty-app (Macroquad, input, rendering, audio, platform saves)
  |                    |
  v                    v
bounty-ai -------------> bounty-core <------------- bounty-tools
                        |
                        v
                 validated content
```

**T-ARC-001.** Begin with four workspace crates: `bounty-core`, `bounty-ai`, `bounty-app`, `bounty-tools`. Content
loading and save encoding can initially be modules of core rather than extra crates. Dependencies must be acyclic. No
core or AI dependency may initialise graphics, audio or a window.

**T-ARC-002.** Core owns rules and authoritative state. App converts input into commands, submits commands, then
presents returned events. Tools call the same core entry points. AI chooses commands; it does not directly mutate state.

**T-ARC-003.** Do not build an ECS, plugin framework or general game engine first. Typed structs, enums, small indexed
collections and explicit state machines are sufficient starting abstractions. Introduce additional infrastructure only
for a demonstrated problem.

## Repository integration

Use the existing repository root workspace and lockfile. Proposed game-relative tree:

```text
bounty/
  README.md
  AGENTS.md                    # local supplement; parent remains authoritative
  crates/{bounty-core,bounty-ai,bounty-app,bounty-tools}/
  docs/spec/
  prototype/
  content/demo/
  balance/
  schemas/
```

Register the four package paths in the existing root workspace, without replacing its resolver, profiles, dependencies,
lints or other games. Only bounty-app is a public binary release. bounty-core, bounty-ai and bounty-tools have
`publish = false`; bounty-app may also use `publish = false` when the repository does not publish Cargo packages. Cargo
registry publication is not the same as distributing an executable [S12].

## Shared kit boundary

**T-ARC-004.** `gamekit` is the repository's shared crate for what every game needs around its rules. Only `bounty-app`
depends on it. `bounty-core`, `bounty-ai` and `bounty-tools` never do: they must build and test with no window, sound
device or kit.

| From `gamekit` | Stays in Bounty |
|---|---|
| 1280 x 720 virtual screen, letterboxing, text drawing and measuring | Board and map drawing, tokens, tiles |
| Basic widgets: button, panel, wrapped and fitted text | Board and map cursor, action menu |
| Translations: Fluent bundles, language table, key-scan test helper | The `.ftl` files and the key test itself |
| Data folder and safe file write (temporary file, then replace) | Save envelope, payload, digest, previous-save backup policy |
| Sound bank: load and play generated WAV data | Which cues exist and how they are synthesised |
| Crash report, developer hooks (open a screen, scripted keys, screenshot) | Screen names and scenario loading |
| The dedication: its text in both languages and the screen that shows it, on the first run and in About | Where About sits in the menu |
| Self-updater (repository, game name and public key as parameters) | Nothing; unused until there is a release |

Random numbers, rule state and saves are never shared between games. Where the kit has no module yet, write it in the
kit when a second game would use it unchanged, otherwise in `bounty-app`.

Do not create another top-level workspace or checked-in lockfile underneath the game. Reuse parent UI, save, audio or CI
helpers only after inspecting their actual contracts. No such reuse is assumed in this specification.

## State model

**T-DAT-001.** `CampaignState` contains rules/content identity, seed, region maps, dynamic object states, hero, army,
contracts, progress, knowledge, clock, status, optional battle and simulation RNG streams. `BattleState` contains battle
identity, encounter participants, board, stable stack IDs, health, action/round flags, pending activation queues, spell
use, battle RNG and outcome.

**T-DAT-002.** Definitions and runtime state must be separate. `TroopDef` holds static statistics, ability IDs, display
keys and asset references. `StackState` holds troop ID, count and changing state. Do not copy definition values into
saves unless the save intentionally embeds a complete, versioned content snapshot.

**T-DAT-003.** Use durable, language-independent IDs for troops, spells, objects, regions, effects and stacks. Catalogue
reordering must not reinterpret a save. Runtime indexing may use compact integers after an explicitly validated
ID-to-index mapping.

**T-DAT-004.** Use integers for rule-significant values. Propose u32 counts, u64 money and health intermediates, and
checked conversions. Coordinate dimensions must be bounded before allocation. Signed intermediate deltas are allowed but
cannot silently wrap into unsigned balances. Never use floating-point distance or presentation interpolation to decide
legal moves or damage.

**T-DAT-005.** Prefer deriving values to storing competing copies. For a health-pool representation, living count is
derived from health and per-creature maximum health; if count and injury are both stored, validate their relationship at
every serialisation boundary. The selected rule determines the calculation, not the renderer.

## Commands, events and atomicity

**T-CMD-001.** A command names actor, intended action, targets and expected state revision. It does not contain a
trusted final price, damage value or reward. The rules compute those values.

**T-CMD-002.** Conceptual API: `apply(state, command, rules) -> Result<Transition, RuleError>`, where a Transition
contains the next state and ordered events. This describes a contract, not an included Rust function. Implementations
may mutate internally, provided failure has no observable partial mutation.

**T-CMD-003.** Validate before commitment. A failure preserves the state revision, all resources, RNG states and event
log. A stale UI command returns a stale-state error and refreshes the view. Never silently retarget it to a different
stack occupying the same cell.

**T-CMD-004.** Events are typed data such as StackMoved, DamageResolved, StackRemoved, SpellConsumed, DayAdvanced,
BudgetSettled, ClueRevealed and CampaignEnded. They contain stable IDs and values, not already-localised prose.
Rendering an event twice cannot apply it twice.

**T-CMD-005.** A sequence of animations visualises an already-committed transition. Animation speed, frame rate and
skipped effects do not influence subsequent state. Input is either held until a suitable boundary or explicitly queued
with revalidation.

## Randomness and determinism

**T-RNG-001.** Select and version an exact PRNG algorithm and sampling procedure. Save its complete state, not just the
original seed. Keep a test vector for each supported algorithm. The prototype uses the fully specified test PRNG in its
own document; this makes no DOS-compatibility claim.

**T-RNG-002.** Separate world-generation, campaign-event, battle, AI-policy and presentation randomness. An AI rollout
never draws from the authoritative battle stream. A spark, sound variant, hover or debug panel never draws from a
gameplay stream.

**T-RNG-003.** Iterate game entities and legal actions in a defined order. Do not rely on hash-map iteration, OS thread
scheduling or frame timing for rule outcomes. Tie-break explicitly by stable IDs or a documented private policy random
draw.

**T-RNG-004.** Reproducibility means the same initial state, rules/content hashes, random algorithm/state and ordered
commands produce the same canonical state on supported builds. A seed alone does not promise compatibility across future
ruleset changes.

**T-RNG-005.** Give AI a fixed expansion/sample budget, not a wall-clock deadline that changes its decision on faster
machines. For responsiveness, suspend and resume search in work chunks without changing the total deterministic budget.
A user cancellation is a separate event.

## Observations and information boundaries

**T-OBS-001.** Define observations as explicit owned projections. Do not pass `&CampaignState` or the true RNG state to
an ordinary agent and rely on discipline. Only information legitimately available to that side belongs in the
observation.

**T-OBS-002.** A battle observation includes visible board, visible stack information, public rules, own permitted
resources and legal actions. It excludes future random outcomes, hidden reward data and unrelated campaign secrets. The
test pilot controlling the player's army has different resources from an enemy troop controller.

**T-OBS-003.** A planning model is built from known information and independently sampled uncertainty. Even when cloning
a small state is convenient, do not clone hidden authoritative values into the agent's search model. Tests should mutate
secrets while keeping the observation identical and verify an identical decision distribution.

**T-OBS-004.** Debug inspection is a privileged development feature. It must be clearly separated from player tooltips
and production AI inputs. Diagnostic files may contain spoilers and should be labelled accordingly.

## Data and validation

**T-CNT-001.** Prefer JSON for the first content format: stable IDs, schema version and explicit units. Each numeric
rule entry needs provenance in a separate register. Do not silently apply a default intended for Prototype to a missing
Classic field.

**T-CNT-002.** Validate unique IDs, referential integrity, bounds, positive health, damage-range order, legal ability
parameters, movement data, map dimensions, spawn legality, objective reachability under the intended unlocks and all
localisation keys. An unknown ability is an error.

**T-CNT-003.** Each map has a coordinate-space tag and conversion version. A content hash covers rule-significant data
and layout, while a separate presentation hash can cover art. Changing an icon should not invalidate a mechanical replay
unless the pack policy explicitly couples them.

**T-CNT-004.** Avoid copying a large data catalogue before there is a validation path. The first useful content pipeline
is a tiny hand-authored scenario that either loads cleanly or reports its exact error.

## Saves

**T-SAV-001.** A save envelope contains format version, application build, ruleset ID/version, content hash, RNG
algorithm IDs/states, campaign state, current battle and policy state. Include a checksum for corruption detection, not
as a promise of anti-cheat security.

**T-SAV-002.** Save only committed gameplay state. Presentation tween progress need not persist; the loaded scene can
show the latest committed position. Pending decisions, round queues, one-use flags and current actor do persist.

**T-SAV-003.** Desktop save flow: encode and validate; write a temporary file in the destination directory; flush as
supported; replace the destination atomically where supported; preserve a previous valid generation. The platform
adapter must test its actual filesystem behaviour. A failure leaves a recoverable save and a useful error.

**T-SAV-004.** During development, support the current save schema and reject incompatible schemas without altering the
source file. Do not build a general migration framework before a supported old format exists. After a public format is
promised, any supported migration is explicit and fixture-tested. Never hide a changed combat formula or missing RNG
state with defaults.

**T-SAV-005.** A future browser adapter must use supported browser persistence and offer export/import. Do not assume
desktop filesystem APIs work in WebAssembly. Browser storage failure must be visible and must not be reported as a
successful save.

## Replay and diagnostics

**T-REP-001.** Replays contain initial state or content-addressed start descriptor, rule/content identity, ordered
accepted commands and checkpoints. Include chosen AI commands; replaying a match must not require rerunning an old AI
search implementation.

**T-REP-002.** Hash a canonical gameplay serialisation with a stable documented digest. Exclude UI, wall-clock
timestamps and presentation RNG. On mismatch, report the first divergent command and relevant fields, not merely a final
unequal checksum.

**T-REP-003.** A test bug report should include the seed, scenario ID, versions, minimal command prefix, expected/actual
transition and permitted diagnostic save. Do not upload user files automatically.

## Quality and performance

**T-QA-001.** CI runs formatting, linting, unit/integration/property tests, data validation and deterministic replay
fixtures and the active balance smoke suite from M1. Desktop build jobs and a browser smoke build are added when those
targets are claimed. Tests use isolated save directories.

**T-QA-002.** Measure before optimising. The first performance target is a responsive UI while AI searches and fast
headless batches; record hardware and build mode with timings. Do not promise an arbitrary games-per-second figure
before measuring.

**T-QA-003.** Asset and save parsers have size limits and reject path traversal, out-of-range coordinates and resource
explosions. A convenient content loader should not trust unbounded external JSON or archive paths.

## Balance tools and artifact identity

**T-QA-004.** Add a small deterministic batch test when the first battle runs. Compare raw results on gameplay changes;
establish the reference-profile baseline before KB tuning. Do not build a synthetic benchmark-management platform in M0.

**T-QA-005.** A no-behaviour-change refactor preserves the fixed corpus outcomes. An intended rule/AI change records its
expected effect and reviews observed deltas. The absence of a measured baseline is explicit, never a successful
comparison.

**T-QA-006.** Keep engine correctness, reference fidelity and balance distinct. Synthetic fixtures remain useful even
after the real-data subset exists, but their loss rates do not calibrate the real game's difficulty.

## Chosen encodings (new in v0.3)

Map authoring uses UTF-8 JSON `map-v1`; see schemas/map.schema.json and the complete content/demo/map.json. Coordinates
are top-left-origin integer cells. Rows are ASCII terrain symbols; objects are a separate typed array. Converted DOS
maps use the same internal model, without forcing original layout into the demo dimensions.

Saves use UTF-8 JSON `save_schema_version: 1` with a checksum over the canonical payload, not over prettified envelope
bytes. ruleset ID/version, content SHA-256, all simulation RNG states, policy state and authoritative campaign/battle
state belong in the payload. Checksum, wall time and application build label are outside the hashed gameplay payload.
The complete state schema grows with implemented systems; loaders reject missing fields for those systems rather than
default them.

Use `bounty-cjson-v1` and SHA-256 as defined in SERIALIZATION.md. This deliberately restricted format is a project
contract, not a claim of RFC-8785 conformance. Golden vectors and the small reference encoder are included. Do not
depend on HashMap iteration or a library's incidental formatting.

Use JSON for benchmark reports too. Do not introduce a binary save format, SQL store, worker service or separate schema
registry for the first game slice.
