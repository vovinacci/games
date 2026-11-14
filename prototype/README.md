# Prototype v0.1: deliberately synthetic battle rules

This profile is an original engineering testbed, **not the statistics or complete rules of DOS King's Bounty**. Its
purpose is to make M1 implementable immediately without pretending the historical audit is finished. Everything in this
file is a proposed prototype rule.

`rules.json` is the data contract. `battles.json` contains four input scenarios. `expected_cases.json` contains
mathematical and command-level expectations. A Rust runner is not included. The expected cases have not been executed by
a game engine.

## Board and participants

The board is six columns by five rows, with coordinates x=0..5 and y=0..4. Each side can have at most five stacks. Every
live stack has a stable ID, side, slot, troop ID, cell, count, residual injury, ammunition and activation flags. At most
one live stack occupies a cell. Obstacles are impassable ground and cannot be landing cells.

The first battle round activates player slots in ascending order, then enemy slots in ascending order. Each living stack
appears once in the normal queue. Its activation begins with the movement budget from its troop definition. A stable ID,
not a mutable vector position, identifies its queued activation.

## Health and damage

For count n, maximum health per creature h and residual injury i: H = n*h-i, where n>0 and 0<=i<h. Dead stacks have n=0,
H=0 and i=0.

Subtract damage from H, clamping at zero. If the result H' is positive, n' = ceil(H'/h) and i' = n'*h-H'. Otherwise
remove the stack from occupancy and every pending queue.

An ordinary attack draws **one** inclusive integer r from that attack type's damage range and deals r multiplied by the
attacker's current creature count. There is no skill multiplier, armour, morale, immunity or range penalty in this
prototype. Use checked u64 intermediate arithmetic.

Partial injury persists throughout a battle. At battle exit it is healed, but creature losses remain. This rule is
synthetic and must not be inferred to describe Classic.

## Primitive actions

**Walk:** move one cell to any of eight neighbours for one movement point. Destination must be free and passable. For a
diagonal, both intervening orthogonal cells must also be free and passable. Walking retains the activation while points
remain.

**Melee:** attack an adjacent enemy in any of eight directions with at least one movement point remaining. Attack ends
the activation. If the defender survives and has not retaliated this round, it immediately makes one melee counterattack
using its surviving count and consumes its retaliation flag. Counterattacks never trigger counter-counterattacks. A
lethal first hit prevents retaliation.

**Shoot:** requires positive ammunition, at least one movement point and no adjacent enemy in any direction. It can
target any enemy on the board; this prototype has no line-of-sight rule. Spend one shot and end the activation. Shooting
never triggers retaliation. Ammunition is set at battle start and never refreshes between rounds.

**Fly:** a flying troop can land on any free passable cell on the board, spending one movement point and its one flight
allowance for this activation. Intermediate terrain and units do not block the flight. Flight does not itself end the
activation. Landing in place is invalid.

**Wait:** before any movement in the activation, defer the stack to the end-of-round deferred queue and keep its full
movement budget. Process deferred entries in insertion order after the normal queue. A second wait by that stack becomes
Pass. Waiting after movement is invalid.

**Pass:** end the activation immediately.

At the next round, reset retaliation, waiting, movement and flight allowances as appropriate; do not reset ammunition.
Refresh the player hero's per-round spell allowance. A dead stack never re-enters a queue.

## Spells

Only the player side has a hero spell resource in this prototype. Hero casting is permitted during a living player's
activation, at most once in a round, provided a charge exists. A legal cast consumes one charge and the round allowance,
but not the stack's activation or movement. Cancelling target selection changes nothing.

**Blast:** choose one living enemy stack; deal 18 damage. No retaliation occurs.

**Mend:** choose one living friendly stack with residual injury; remove up to 12 injury. It does not restore dead
creatures or a dead stack. A fully healthy target is invalid and spends nothing.

Both spells ignore distance and obstacles. Enemy policies may not cast them. For a symmetric agent-v-agent benchmark,
disable hero magic for both sides explicitly rather than quietly granting an enemy hero.

## Ending and stalling

After each damage/effect transaction, if one side has no living stacks, the other wins. A destroyed defender cannot
retaliate before this check. This prototype's damage order cannot produce simultaneous elimination, but malformed
external fixtures should be rejected rather than inventing a winner.

After 200 completed rounds without a normal result, the test runner returns `budget_exceeded`. This is a diagnostic
outcome, not an ordinary player defeat or a Classic rule. Interactive handling follows G-CMB-011.

## Prototype random generator

Use a u32 state. A raw draw updates it to `(1664525 * state + 1013904223) mod 2^32` and returns the new state. Seed zero
is valid.

To sample inclusive [lo, hi], require lo<=hi and span=hi-lo+1<=2^32. Compute `limit=2^32-(2^32 mod span)` in u64. Draw
until raw<limit, then return lo+(raw mod span). The implementation must not overflow computing the limit. For span=1,
return lo **without consuming a raw draw**.

This simple, fully specified test generator is not a claim of statistical suitability for every future system and does
not match the DOS RNG by assertion. Version it as `prototype-lcg32-v1`. Changing it changes the ruleset version and
golden replay results. AI planning has a separate generator state and cannot inspect the battle generator.

Forced damage rolls in `expected_cases.json` are a test-double input, not a production command that players or agents
can submit. Validate each forced roll lies inside the permitted range.

## Recruitment arithmetic example

A prototype recruitment test uses a per-stack nominal health limit: existing count plus recruited count, multiplied by
creature maximum health, must be <= leadership. Affordability allows spending the exact remaining funds. A full
different-type army cannot recruit another stack. These are prototype decisions; Classic equality and control behaviour
remain U-06/U-07.

## Use after review

The synthetic rules above are unchanged. They test the engine, not KB balance, historical fidelity or whether KB combat
is fun. Do not tune the intended KB AI on this troop catalogue.

A complete synthetic expedition is now supplied in content/demo/expedition.json, with a map, command routes and derived
expectations. See its README for timing, reward, recovery and deadline ordering. A source-backed KB subset remains a
separate step before reference-game playtesting and balance calibration.
