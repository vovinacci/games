# 02. Game specification

## Interpretation

All requirements in this document are proposed implementation requirements (D). Their exact historical parameters come
from `07-RULES-REGISTER.md`, not from the author's memory. The synthetic prototype is an engineering test contract, not
a KB balance model. Reference mechanics may be adopted provisionally from a pinned source under 07-RULES-REGISTER.md;
exact-DOS claims require executable evidence.

## Screens and state transitions

**G-FLOW-001.** The application MUST support Title, New Campaign, Adventure, Battle, Result and Settings screens. Hero,
Army, Spellbook, Contract, Puzzle, Location and Save/Load are modal views over a stable gameplay state.

**G-FLOW-002.** New Campaign MUST select name, class, campaign difficulty, ruleset, AI policy and seed. Do not hide an
AI change inside a difficulty name. Before confirmation, show the selected time allowance and rules/profile identifiers.
Invalid or unsupported content blocks campaign creation with an actionable message.

**G-FLOW-003.** A modal interaction MUST either cancel without mutation or submit a complete validated command.
Switching views, changing language, resizing and adjusting sound MUST NOT advance game time or consume randomness.

**G-FLOW-004.** Each screen MUST provide keyboard operation. Mouse operation MAY add convenience but cannot be the only
way to choose a legal target. Focus, selection and confirmation must be visibly distinct.

## Campaign and movement

**G-WLD-001.** Represent terrain, locations, dynamic encounters and player knowledge separately. A cleared encounter
must not destroy the underlying terrain or permanently reveal unrelated objects.

**G-WLD-002.** Store a continent/region ID and integer tile coordinates. Use one internal coordinate convention: origin
at the top-left, x rightwards, y downwards. Any original map coordinates must pass through a tested conversion adapter.
Never mix original reference coordinates directly into screen coordinates.

**G-WLD-003.** The movement resolver MUST validate origin, destination, movement mode, terrain, occupancy, transition
conditions and time cost. It returns an ordered event sequence. A rejected command consumes neither time nor resources
nor random draws.

**G-WLD-004.** Support land, sailing and flight as explicit modes with explicit eligibility predicates. A temporary
ability expiring while the army is in an otherwise invalid location needs a defined resolution, not a panic or free
teleport. Resolve the Classic case in U-04.

**G-WLD-005.** A region transition MUST validate access, destination and vehicle state. Cancelled transitions leave the
player unchanged. Arrival must not place the army outside the map or inside a blocking object.

**G-WLD-006.** Exploration MUST update a persistent knowledge map. The visible scene and minimap are projections of
knowledge, not unrestricted renderings of the entire world state. Location information acquired by an explicit game
effect can update knowledge without movement.

**G-WLD-007.** Optional route following MUST use known information, stop before an encounter or meaningful choice, and
revalidate each step. It must stop if a route becomes invalid. Display the planned route and expected known cost, but do
not pretend that unknown travel costs are exact.

**G-WLD-008.** World enemies MUST have a separate update policy from battle enemies. Define when they take a step,
whether they pursue, how ties resolve and what happens when paths are blocked. Do not introduce real-time wandering
while the player is reading a menu.

## Time and finance

**G-TIM-001.** A single campaign clock MUST own elapsed time, day boundaries, week boundaries and the deadline. Every
time-consuming command uses this clock; no screen may decrement days independently.

**G-TIM-002.** Crossing several boundaries MUST process each applicable boundary once in chronological order. Waiting,
searching, travelling and recovery must use the same mechanism. Test actions ending exactly on a boundary, crossing
multiple boundaries and reaching the final day.

**G-TIM-003.** A temporary time-changing effect MUST define what it freezes: player travel cost, enemy movement,
boundary events, effect duration, or some combination. Do not treat one vague `time_stopped` flag as a complete rule.

**G-ECO-001.** All money flows MUST have a ledger event: starting funds, income, purchase, recruitment, reward, upkeep,
transport and special effects. Distinguish a one-time cost from a recurring obligation.

**G-ECO-002.** Weekly settlement MUST explicitly define its order, shortage behaviour and temporary-effect expiry. The
UI must show a forecast from the same computation used for settlement, with unknown future changes excluded or labelled.

**G-ECO-003.** Purchases MUST be transactional. Validate quantity, stock, affordability, capacity and location
permissions before changing any of them. Equality cases are mandatory tests. Use bounded integer arithmetic and checked
intermediates.

**G-ECO-004.** Do not invent enemy economic income or army production to support a more sophisticated AI. New campaign
simulation would be a separate design project.

## Hero, army and progression

**G-ARM-001.** A hero MUST store class, rank, leadership, funds, spell capability, spell power, spell inventory,
artefacts, campaign flags and progress. Derived values must be computed from base values plus explicit modifiers, not
repeatedly multiplied into saved base fields.

**G-ARM-002.** An army MUST use stable stack IDs and a ruleset-defined capacity. Recruitment, joining, dismissal,
reordering and garrison transfer require named operations and validation. Do not assume arbitrary stack splitting is an
original feature; U-07 decides the permitted operations.

**G-ARM-003.** Leadership evaluation MUST return both status and an explanation. An interface warning and the combat
resolver must use the same predicate. Recruitment limits and involuntary growth are separate cases; avoiding oversize
purchases does not eliminate the need to handle a stack that becomes uncontrolled later.

**G-ARM-004.** Morale MUST be computed through an auditable rule with explicit inputs. The display, AI and resolution
must agree about its effect. Do not introduce a generic RPG happiness statistic as a substitute.

**G-ARM-005.** Promotion MUST check prerequisites, apply rank changes exactly once and emit a summary. Loading,
reopening a location or repeating a dialogue must not duplicate promotion bonuses.

**G-ARM-006.** Garrison transfers MUST preserve counts and ownership. A transfer failure cannot lose troops. Recruitment
and garrison stock must not be conflated.

## Locations, contracts and progress

**G-LOC-001.** Implement location interactions as typed services: audience/promotion, recruitment, garrison, contract
handling, information, transport, spell purchase, training, siege equipment and special travel. Each service declares
its prerequisites, cost, effects and cancellation behaviour.

**G-LOC-002.** Treasure and special objects MUST have persistent stable IDs and one-shot collection state. Generate
their hidden outcome at a specified point and save it. Previewing, closing a dialog or changing language must not reroll
the reward.

**G-LOC-003.** The reference campaign requires a catalogue of region layouts, location definitions, encounter
populations, treasures and special objects. Exact data must include provenance and conversion rules. A loader MUST
reject missing links rather than substitute unrelated prototype content.

**G-QST-001.** Model contract ownership separately from a villain's current location and defeat status. Encounter
outcome resolution must decide rewards, capture/escape, relocation and clue entitlement through one atomic transaction.
U-14 defines the Classic details.

**G-QST-002.** Track objective location separately from player knowledge. A clue reveal changes knowledge, not the
objective itself. Reopening the puzzle or changing its display scale cannot reveal additional information.

**G-QST-003.** The puzzle view MUST render a reproducible terrain excerpt and a reveal mask. Unrevealed cells must not
leak through hover, screen-reader labels, minimaps, logs or save-file previews. The authoritative save may contain
secrets; normal gameplay UI must not expose them.

**G-QST-004.** Searching MUST validate the tile and charge the configured cost. The ability to search must not require
every clue unless the selected rule explicitly does. Successful search, failed search and deadline expiry need a tested
ordering.

## Combat

**G-CMB-001.** Use a square-cell battlefield. The reference reconstruction uses six columns and five rows; this is
corroborated by the reimplementation headers, not measured from an original executable in this draft. [S4] The prototype
uses the same board dimensions.

**G-CMB-002.** A battle begins from an immutable encounter descriptor: location, participants, troop counts, layout,
ruleset, modifiers, battle identity and RNG initialisation policy. Battle setup must not consume or destroy campaign
units before a committed start.

**G-CMB-003.** The rules MUST explicitly define initial placement, activation order, round boundaries, movement
neighbourhood, collision, attacks, ranged eligibility, flight, waiting, passing, retaliation and end conditions. Use the
complete prototype definitions while U-08 through U-12 are unresolved for Classic.

**G-CMB-004.** Legal actions MUST be generated by the simulation, not reconstructed independently by the UI and each AI.
At minimum the API can represent move, melee, ranged attack, flight, wait, pass, spell and surrender; a ruleset may
disable individual actions.

**G-CMB-005.** Damage MUST resolve through a shared calculation with a structured breakdown: base amount, relevant
modifiers, rounding steps, protection, losses, remaining health and secondary effects. A combat log may present a
simpler explanation, but debug output must preserve the breakdown.

**G-CMB-006.** Separate creature count, per-creature maximum health and residual injury. Explicitly define which health
state persists through activation changes, healing and battle exit. Do not change this representation to make a
preferred formula easier.

**G-CMB-007.** Retaliation eligibility, waiting status and other round flags MUST reset at an explicit phase boundary.
Stable stack IDs survive casualty removal. Removing a stack must not cause a later stack to receive the dead stack's
pending action.

**G-CMB-008.** Special abilities MUST be named, testable rules with parameters, not conditions on translated creature
names. Every ability needs ordering tests when combined with damage, death, retaliation, healing and control changes.

**G-CMB-009.** Field and siege battles MUST use explicit layout/eligibility policies. Do not model siege machinery as an
extra tactical unit without source evidence. Entry conditions and battlefield obstruction are different concerns.

**G-CMB-010.** Battle completion MUST write back once: survivors, persistent injury according to policy, losses,
rewards, enemy state, ownership and follow-on campaign events. Skipping animations or loading the result screen must not
duplicate the reward.

**G-CMB-011.** A watchdog MUST detect stalled AI or pathological loops. The development harness reports a distinct
`stalled`/`budget_exceeded` result; it must not count that as an ordinary defeat. The interactive game preserves a
diagnostic save and offers recovery rather than secretly awarding victory.

## Magic and exceptional effects

**G-MAG-001.** Each spell MUST declare its phase, ownership/capability requirement, charge cost, target kind, legality
predicate, effect, duration, stacking rule and removal rule. A target-selection cancel spends nothing.

**G-MAG-002.** Hero spells and creature abilities MUST be distinct. An enemy policy cannot acquire a hero spellbook
merely because the test harness supports symmetric armies. Eligibility is a rule, not an AI feature.

**G-MAG-003.** Centralise effect ordering. An effect transaction validates targets, consumes the permitted resource,
computes effects, resolves deaths/control changes, checks battle termination and emits events. A no-effect but legal
cast needs a defined charge policy.

**G-MAG-004.** Travel-changing, time-changing, army-generating and health-changing magic require campaign invariants as
well as battle tests. Test expiry and capacity changes during save/load and week boundaries.

## Defeat, victory and scoring

**G-END-001.** Separate battle defeat, surrender, campaign timeout and campaign victory. Defeat recovery requires a
documented policy for location, troops, gold, transport, time and retained progress. The historical policy is U-15, not
"delete the save" by assumption.

**G-END-002.** Terminal campaign state MUST be persistent and idempotent. A result screen cannot award rewards again.
Provide a final summary, the seed, rules/policy versions and an explicit new-campaign action.

**G-END-003.** Scoring is a data-backed calculation with a breakdown. It is not the sole balance metric. Do not use
score to conceal an unwinnable or stalled campaign.

## Interface, presentation and accessibility

**G-UX-001.** Adventure shows current region, position context, remaining time, funds, army and contract access, and
available actions. Battle shows active stack, ownership, health/count, movement, legal targets, spell availability and
the event log.

**G-UX-002.** Provide movement previews, clear costs and legality explanations. Distinguish confirmed information,
estimates and unknowns. A forecast cannot call the authoritative random generator.

**G-UX-003.** Require confirmation for surrender, dismissal and irreversible high-cost operations. Routine movement and
attacks should not create confirmation-dialog fatigue. Cancelling an uncommitted action is always safe; gameplay undo is
not assumed.

**G-UX-004.** Use a scalable virtual canvas with readable text. Use the 1280x720 layout and 32-unit adventure tiles
specified in CONTENT-AND-ASSETS.md; these are remake choices, not DOS-resolution claims. Test at small window sizes,
high display scaling and both languages. Keep pixel-art filtering separate from text readability.

**G-UX-005.** MVP: fixed documented keyboard controls, visible shortcuts, audio on/off and normal/instant animation. A
remapping UI and configurable filters are deferred. Important state must not be communicated solely by colour, sound or
motion. Essential information must remain available when animations are disabled.

**G-UX-006.** Text is localised by stable keys with parameters. All public screens must work with longer translations
and plural forms. Names and unit identifiers in saves must remain language-independent.

## Saves and tools

**G-SAV-001.** Save at committed decision boundaries, including during a battle. Persist all rule-significant state and
RNG state. A user must not need to finish a battle merely to close the program safely.

**G-SAV-002.** MVP: one explicit Save/Continue slot plus one previous valid backup. Save at committed decision
boundaries, including mid-battle. Validate a new save before replacing the previous one. Multiple slots and rotating
autosaves are deferred; do not claim an inherited implementation exists.

**G-SAV-003.** Invalid, incompatible or corrupt saves MUST produce a recoverable error. Preserve the original bytes.
Content mismatches must not silently change the meaning of troop or spell IDs.

**G-DEV-001.** Provide deterministic scenario loading, command replay, battle benchmarks, state inspection and UI
screenshot hooks. These are development requirements, not public cheat-menu requirements.

**G-DEV-002.** Public packages MUST include a content/license manifest and identify known deviations. Do not include
copied assets merely because the executable is distributed for free.

## Balance-development contract

**G-DEV-003.** From the first executable headless battle, store fixed-scenario, fixed-seed outcomes and compare affected
metrics on gameplay changes. Build this small runner with the simulation; no M0 comparator framework or automatic
balance approval is required. See 09-BALANCE-REGRESSION.md.
