# 04. AI and balance

## Purpose

**A-GOAL-001.** The AI is part of the game's balance. The objective is not to maximise an isolated win rate; it is to
produce understandable tactical pressure that makes recruitment, positioning, casualty control and campaign planning
meaningful.

**A-GOAL-002.** Keep four different responsibilities distinct: enemy battle policy; wandering-enemy campaign policy;
automated player-side test pilot; optional player auto-battle. A sophisticated test pilot does not imply that ordinary
enemies can recruit armies, cast hero magic or access the player's campaign secrets.

**A-GOAL-003.** Do not add a competing strategic empire merely to give the AI more work. The interesting first problem
is a good tactical opponent under the existing rules.

## Engineering agent versus balance agent

The synthetic fixtures use a deliberately simple deterministic pilot. It tests legality and creates repeatable
engineering results. Do not optimise its weights on the synthetic troop table and call that improved KB AI.

Before tuning the intended opponent, create a reference-rule subset with actual source-backed troop statistics and the
full skill/morale/control/activation rules used by that subset. Pin the provisional semantic source and label unobserved
behaviour. Learning that subset is the first KB-specific balance experiment.

## Policy ladder

| Policy | Role | Initial design |
|---|---|---|
| Random legal | Test oracle for legality and termination, not a shipping difficulty. | Choose from canonically ordered legal actions using a private RNG. |
| Baseline | Explainable development opponent. | Prefer useful damage, otherwise approach a legal target; pass when necessary. |
| Tactical | Intended first enjoyable opponent. | Evaluate complete activation plans, retaliation, exposure and enemy replies. |
| Reference | Optional preservation mode. | Requires observed original behaviour; no claim of exactness beforehand. |
| Expert | Later experiment. | More search and better evaluation, without extra information or resources. |

**A-POL-001.** A policy returns a legal command plus an optional debug explanation. The core revalidates it. An illegal
AI result is a bug, not an excuse for the engine to mutate the board directly. A safe fallback may prevent a crash, but
the occurrence must fail the relevant test.

**A-POL-002.** Personality is not difficulty. Aggression, casualty aversion, positional preference and target preference
are evaluation parameters; search effort is a separate setting. Keep a few named profiles rather than randomising every
weight each turn.

## Tactical planning

**A-PLAN-001.** Begin by evaluating short activation plans: movement followed by attack, a ranged shot, a flight/landing
followed by a permitted action, waiting, or passing. Convert the chosen plan to primitive commands and revalidate after
each transition. This avoids judging a necessary movement step as useless because it causes no immediate damage.

**A-PLAN-002.** Rank candidates using a fixed integer evaluation. Proposed terms are: enemy combat power removed; own
power retained; finishing a dangerous stack; suppressing a shooter; creating or escaping a credible threat; preserving
safe movement; avoiding useless overkill; and cost of a consumable for player-side automation.

Do not equate one expensive creature with every cheaper creature purely by shop price. Evaluation should consider
surviving damage output, reach and useful abilities. Economic replacement cost may be an additional campaign-sensitive
term for the player pilot, not the whole tactical value.

**A-PLAN-003.** After the reference-rule baseline exists, the first tactical policy SHOULD examine the opponent's
strongest few plausible replies. A deterministic candidate ordering and node budget make tests reproducible. Sample
uncertain damage from a planning-only RNG or compute expected outcomes; never read the next actual combat roll.

**A-PLAN-004.** Do not give the enemy access to player-only spell inventory or future human actions. When a reply model
needs unknown information, use a documented assumption or distribution. Information assumptions belong in the policy
version.

**A-PLAN-005.** A planner must reason about opportunity cost: consuming an opponent's retaliation can help a later
attacker; moving next to a shooter may be valuable even without an immediate kill; waiting can be useful but must not
create an endless loop. The exact legality comes from the chosen ruleset, not duplicated AI code.

## Difficulty and fairness

**A-DIF-001.** Separate campaign pressure from tactical strength in setup and save metadata. A campaign-difficulty
profile can alter only the parameters the chosen rules permit. An AI profile alters decisions, not troop statistics,
finances or random outcomes.

**A-DIF-002.** No hidden stat bonuses, access to future random rolls, secret knowledge or rubber-banding in the first
release. Deliberate asymmetries already present in the game remain explicit content; this is not a requirement that
every encounter be a fair mirror match.

**A-DIF-003.** An easier policy can search less, prefer immediate gains or choose among near-best moves with bounded
private randomness. It must still obey rules and should not make conspicuously nonsensical moves simply to lose.

**A-DIF-004.** A stronger policy changes effective campaign difficulty even when all statistics remain fixed. Record and
test this change. Rebalance an experimental preset or provide a different policy selection; do not silently alter
Classic creature values to hide it.

## Minimal first pilot

`approach-v1` has no search weights. During each activation: take a legal melee attack if any; otherwise take a legal
shot if any; otherwise move toward a reachable cell adjacent to an enemy. Attack ties use target stable ID. Ground
routes use shortest path under actual movement rules with neighbours ordered by (y,x); choose equal-distance goals by
target ID then (y,x). Fly only when available and it shortens that route; choose the landing by the same ordering. If no
reachable attack position exists, Pass. Never use Wait or hero spells. Recompute after every command. The loop is
bounded at 64 commands per activation; exhaustion is an error, not a hidden extra turn. Later changes require a new
pilot version. This is a transparent baseline, not an assertion of intelligent play.

## Benchmarks

**A-BEN-001.** Maintain a small, fixed tactical suite from M1, before running large tournaments: melee exchanges,
retaliation bait, blocked shooter, ranged duel, flight around obstruction, wait timing, a weak-stack sacrifice, an
unwinnable encounter, a siege bottleneck and a legal spell decision for a player-side pilot.

**A-BEN-002.** Compare policies using paired scenario seeds and swapped sides where symmetry makes that meaningful. Also
include asymmetric campaign encounters. Do not report a single aggregate win rate as proof of better play.

**A-BEN-003.** Record: wins/losses/stalls separately; survivors and replacement cost; spell consumption; rounds and
actions; illegal choices; search effort; time per decision; and losses in scenarios that should be comfortably winnable.
Include the exact scenario/content and policy versions.

**A-BEN-004.** Reserve a hold-out suite not used to tune weights. Tune on one set, then evaluate on the other. Otherwise
a policy can merely memorise the situations used to praise it.

**A-BEN-005.** Campaign simulation uses a player-side test pilot with its own declared information and objectives. It is
a tool for detecting economic traps, unreachable progress and pathological casualties. Its completion rate is not a
substitute for human playtesting.

## Day-one balance regression

`09-BALANCE-REGRESSION.md` is mandatory alongside this document. Benchmarks start with the first complete battle, not
when the Tactical policy is ready. The random and baseline policies are sufficient to expose a changed distribution;
scripted micro-cases provide exact expectations independently of their playing strength.

For a rules/data change, freeze policy implementations and scenarios. For an AI change, freeze rules and content. A
change affecting both requires an explicitly combined comparison and, where compatible, a small factorial diagnostic.
Record troop-loss costs and campaign consequences as well as win rates. Never treat an AI-only tournament score as proof
that campaign balance or human enjoyment is correct.

## Debugging and explanation

**A-DBG-001.** In a development overlay, show the top candidate plans, evaluation terms, sampled outcomes, rejected
plans and chosen tie-break. A compact explanation might say "close on shooter; immediate damage lower, expected reply
losses lower".

**A-DBG-002.** Keep full diagnostics out of ordinary play unless enabled. They must never reveal unrevealed campaign
data. The normal combat log explains resolved events rather than an omniscient prediction of the next turn.

## Acceptance gates

The first tactical policy passes only when it produces no illegal actions in the test suite, cannot read hidden state or
authoritative RNG, terminates within its declared search budget, and improves selected tactical metrics against the
baseline without a major regression elsewhere. Specify those selected metrics before tuning.

There is deliberately no universal "50 percent wins" target. King's Bounty-style campaign encounters are not symmetric
competitive matches. A better measure is whether capable play yields sensible casualty costs and whether the opponent
exploits obvious mistakes without relying on privileged information.
