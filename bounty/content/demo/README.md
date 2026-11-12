# Demo expedition v1 - exact integration contract

Self-authored synthetic content. It tests subsystem integration and report plumbing, not original KB mechanics, economic
tuning or an AI's competence. No engine execution is claimed by the expected results in expedition.json.

Load map.json and prototype-v0.1 battle rules. Begin at the town with 10 Wardens, 100 gold and leadership 120. Only one
Warden army stack is used in this fixture. Recruitment merges into it; nominal count*10 <= 120, stock and exact
affordability are required. Invalid actions are atomic. No enemy wandering, stock refresh, spells, random treasures,
sailing, promotion or morale exists in this demo profile.

The town permits recruitment and taking the demo-captain contract at zero time cost. Entering an uncleared encounter
costs one movement tick and opens its battle; keep the destination as a pending arrival. Victory commits arrival, clears
the object and grants its gold once. No reward on surrender/defeat. Units are placed as specified; their runtime counts
come from the expedition and encounter. Partial injury heals on battle exit under prototype rules. Battle actions
consume no campaign ticks. The two listed encounters are one-hit wins for the recruited 12-Warden army for every
permitted damage roll; no casualty distribution is invented.

Defeating the captain while holding its contract grants the single clue, revealing the objective coordinates. Without
the contract the gold is still awarded, but no clue is awarded. The target never relocates in this fixture. Searching
the right tile can succeed even without a clue; hidden objective data are never presented by ordinary tooltips. An
already-cleared encounter is an ordinary tile.

Every cardinal movement costs one tick. Diagonal movement is disabled in this integration map, independently of
eight-neighbour tactical movement. Reject water and out-of-bounds moves. Searching costs four ticks, requires passable
land and cannot occur during battle. Wait_tick costs one tick. Invalid commands and modal views cost nothing. Map object
interactions other than entry/search cost zero.

Every 20 elapsed ticks, apply income then upkeep once: add 20 gold, charge one per living creature. Insufficient funds
terminate this deliberately simple fixture as insolvent (not a claim about DOS). No further effects occur after that
terminal condition. Normal deadline is elapsed tick 24: a successful search that finishes exactly there wins; any other
action reaching it times out. No action may commit past tick 24: one that would overrun advances only to that boundary,
applies due settlement, then times out without delivering its requested success/reward. A search on the objective begun
too late is therefore not a victory. Weekly events precede a same-boundary search/deadline result. Tests cover equality
explicitly.

Surrender or losing the entire army sends the hero to the town, reduces current gold to floor(gold/2), gives four
Wardens and spends four ticks. Preserve stock, contract/clues and previously cleared encounters. The failed enemy is not
cleared and resets to its original strength. Weekly/deadline processing applies during recovery with the recovered
four-creature army; abort on a terminal boundary. No zero-troop adventuring state remains after a successful recovery.

The `resolve_battle` item in test routes is a harness directive: play legal battle commands with the specified pilot
until a real result. It is never a public core command accepting an externally supplied winner. Surrender remains a real
command. Do not implement a test stub that simply reads the expected winner.

For the happy route: recruitment 40, rewards 90, no weekly boundary, 11 travel ticks plus four search ticks -> 150 gold,
15 ticks, 12 Wardens. For the deadline route, one settlement adds net 8 gold -> 158. These are derived fixture
expectations, not measured simulator outputs. Record actual results once the Rust engine exists.
