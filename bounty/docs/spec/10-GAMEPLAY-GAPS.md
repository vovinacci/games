# 10. Where the original falls short, and what to do about it

A study, not a decision. It asks where King's Bounty (1990) is weaker as a game than it could be, what could be changed
for better play without losing what makes it King's Bounty, and when each change can be tried. Nothing here changes the
reference rules: they stay as the register (`07-RULES-REGISTER.md`) has them.

## How to read it

Every claim carries the kind of evidence behind it:

- **Measured**: battles played by this project's engine under `reference-v0.1`, the pilot `approach-v1` on both sides,
  500 seeds a line. Reproduce with `cargo test --release -p bounty-ai --test gap_study -- --ignored --nocapture`.
- **Rules**: follows from the rules as the reference has them (OpenBounty at the pinned revision; section numbers are
  those of its specification). True of the original only as far as the reference is.
- **Opinion**: a judgement about how it plays, from knowing the game. Not tested here; the weakest kind.

Limits of the measurements, which matter:

- Three troops of 25, so nothing about special abilities, spells, or the top of the catalogue.
- The pilot is plain: it walks up and strikes, never waits, never keeps its shooters away. A result that depends on how
  a troop is played (above all the shooters) may not hold for a person.
- Rules read from a reimplementation, not observed in the DOS game.
- No campaign exists yet, so everything about the campaign is Rules or Opinion.

The original's numbers stay out of this document, as out of the repository, except those of the three troops already
allowed. Where a number would help, the reference's section is named instead.

## What works, and must survive

- **A whole campaign in an evening or two**, against a clock. The clock makes every detour a choice. (Opinion)
- **The army is a puzzle**: five slots, a leadership limit per stack, troops that do not get on. (Rules: sections 13,
  14)
- **Gold or leadership** from a chest: a real choice between now and later, asked dozens of times. (Rules: 22.2)
- **Contracts** give the map direction: someone to hunt, a reward, a piece of the map. (Rules: 21)
- **Battles are quick.** Most are over in a few rounds. (Measured: one to six rounds on average in the cases below,
  except two: eight for a mixed army against its like, sixteen for Archers shooting it out)

A change that costs one of these has to be worth it.

## Battle

### B1. What the army screen says about morale is not what the dice do

- **Rules.** Morale scales damage: Low halves it, High adds half. The reference documents that the label on the army
  screen and the multiplier in battle read the chart in opposite directions, and calls the latter the original's
  "documented bug" (25.5; register, "The two morale rules"). For Sprites with Wolves the screen blames the Wolves; the
  battle halves the Sprites.
- **Measured.** The same army of 60 Sprites and 10 Wolves on both sides: without morale the side moving first wins 58 of
  100; with the hero's morale applied to it, 7. Wolves alone go from 47 to 85 with it (High), Sprites alone from 61 to
  75. Sprites with Archers: no change.
- **So what.** Morale is the largest single lever measured: the same troops win 85 or 7 times in 100 by who stands
  beside them. That is the army puzzle working. The gap is that the player is told the wrong thing about it.
- **Candidates.** (a) Show, for each stack, the morale the battle will apply and why. Information only; no rule changes.
  (b) One rule for label and battle, the "army view" rule. A rule change: it moves the penalty to other troops. (c)
  Soften Low. Not before (a) has been played.

### B2. A stack one creature too large turns on its army

- **Rules.** A stack whose whole health exceeds the hero's leadership fights everyone, its own side first (14.2). Buying
  cannot cause it: the recruiting limit stops at the leadership (13.4). What can is not spelled out in the sections
  read; a band joining for free and leadership falling are the likely ways, to be checked when those are built.
- **Measured.** 20 Wolves and 4 Archers against a weaker enemy: 99 of 100 with leadership 60, exactly their health; 81
  with 59; 69 with 45. An army with its 40 Wolves at twice the leadership: 17, against 93 in control.
- **So what.** One point of leadership is worth eighteen battles in a hundred. Nothing between "fine" and "mutiny".
- **Candidates.** (a) Warn before anything is accepted that would put a stack over, and mark such a stack everywhere,
  not only in battle. Information only. (b) A stack over the limit by little obeys most of the time. A rule change;
  needs a design.

### B3. Price and leadership do not follow strength

- **Rules.** Damage is roll times the number of creatures; leadership limits a stack's whole health; recruiting costs
  per creature. So a troop of little health each puts the most creatures, and the most damage, under a given leadership.
  Across the catalogue, roll per point of health differs about twentyfold, roll per coin about fiftyfold, before skill.
  Skill pulls the other way: the damage term falls with every level the target stands above the attacker and reaches
  nothing at five levels (25.5), so the cheap swarm that rules the first continent cannot hurt the top troops.
- **Measured.** Armies of equal price: 12 Archers lose every one of 100 battles to 75 Wolves and to 200 Sprites, moving
  first or second. Armies of equal leadership: the same. Sprites beat Wolves 68 to 32 at equal price and 83 to 17 at
  equal leadership, whoever moves first.
- **Caution.** The pilot shoots while it can but never moves its Archers away from what comes at them; a person would
  put another stack in front. With one stack a side there is nothing to put in front. The loss is real for this setup;
  how much of it is the troop and how much the play is open.
- **So what.** At each stage of the game one or two troops are the right answer and the rest are traps, some of them the
  expensive ones a new player will reach for. Choices that look open are not.
- **Candidates.** None yet. First measure the whole catalogue (needs the importer), with a pilot that screens shooters.
  Then, in a variant profile, move prices before touching any troop's numbers: prices change what is worth buying
  without changing how a battle goes.

### B4. Walking up to the enemy is paid for, and the dead strike back

- **Rules.** The answer to a blow comes from the number of creatures the target had before it (25.5). Attacking does not
  weaken the answer.
- **Measured.** Equal armies of Wolves: the side that moves first, and so walks into reach, wins 47 of 100. For Sprites,
  who fly the whole way and strike in one turn, 61; for Archers, who shoot first, 69. The winner of an even fight keeps
  22 to 43 in 100 of its creatures (72 for Sprites, whose fights last one round).
- **So what.** Between equal stacks an exchange is a coin toss paid for by both, and reaching the enemy first is no
  advantage for those who walk. Victories are bloody, so every fight sends the player back to recruit.
- **Candidates.** The prototype's rule, the answer from the survivors, as a variant: it rewards striking first and makes
  a won fight cheaper. Measure what it does to these numbers before judging it.

### B5. One roll decides a whole stack's blow

- **Rules.** A blow rolls once and multiplies by the count (25.5). A hundred creatures do not average out: Wolves do
  one, two or three times the count, by one roll.
- **So what.** (Opinion) Large fights swing as much as small ones; a plan that should work fails one time in three on a
  roll.
- **Candidates.** The mean of two rolls, or of several, as a variant: the same average, less swing. Measure how often
  the stronger army loses, before and after.

### B6. The enemy does not think

- **Rules.** The reference's enemy follows a fixed list: strike what is near, else shoot, else fly up, else walk up
  (25.10). So does this project's pilot.
- **So what.** (Opinion) Battles are won when the armies are chosen, not during them. With a small board and such an
  enemy there is little to decide once it starts.
- **Candidates.** The tactical opponent the specification already plans (`04-AI-AND-BALANCE.md`), offered beside the
  reference behaviour, never instead of it.

## Campaign

Nothing here is measured: there is no campaign yet. Each point names what would measure it.

### C1. Upkeep has no teeth

- **Rules.** Each week the commission is paid, then the army's upkeep is taken; a shortfall leaves the purse empty and
  every troop stays (13.4, 23.2). The reference's own second content pack changes this: unpaid stacks leave.
- **So what.** An army too expensive to keep costs nothing more once the purse is empty.
- **Candidates.** Unpaid stacks leave, in slot order, as that pack does. Measure with the expedition pilot: how often
  does a player-like route run dry, and what does it lose.

### C2. Difficulty is only a shorter clock

- **Rules.** Difficulty sets the days and the score multiplier, nothing else: not gold, not armies, not what the map
  holds (8.6).
- **So what.** (Opinion) The hardest setting is the same game played faster, and the easiest leaves the clock
  meaningless.
- **Candidates.** Keep the clock as the lever, since it is the game's own; consider the opponent's play (B6) as a
  second, separate lever, which the specification already keeps apart from difficulty.

### C3. The final search is guessing against the clock

- **Rules.** The sceptre lies under one unmarked tile of open ground somewhere in the world; the map of it is uncovered
  piece by piece; a search costs the same days whether or not anything is found (7.3, 10.6).
- **So what.** (Opinion) With the map half uncovered the end of the game is trying tiles and watching days go.
- **Candidates.** (a) Say what a search would cost before it is made, and show which tiles have been searched.
  Information only. (b) A failed search near the place says so. A rule change, small.

### C4. A villain beaten without his contract is beaten for nothing

- **Rules.** One contract at a time, offered in a fixed rotation of five. Winning a villain's castle without holding his
  contract frees him: garrison rebuilt, the fight to be had again (21.2, 21.3).
- **So what.** (Opinion) Exploring first is punished, and getting the right contract means trips to a town to turn the
  rotation.
- **Candidates.** (a) Say so before the siege. Information only. (b) Let a contract be chosen among those on offer. (c)
  A villain beaten without a contract stays beaten and pays less.

### C5. Getting troops back depends on the week's draw

- **Rules.** A dwelling refills only in a week whose creature is its own; the week's creature is drawn (24.1, 24.2).
  Castle troops are never short.
- **So what.** (Opinion) After a bloody win (B4) the troop wanted may not come back for many weeks, so armies drift to
  castle troops.
- **Candidates.** A slow refill every week besides the draw. Measure with the expedition pilot.

### C6. The world stands still

- **Rules.** Enemies move only when the hero comes within two tiles (15.5). Castles and bands grow a little in their
  creature's week (24.2).
- **So what.** (Opinion) Nothing presses but the clock; the map is a list of fights to take in the best order.
- **Candidates.** None proposed. This is the edge of the game's identity: a world that acts would be another game.

### C7. Travel is keypresses

- **So what.** (Opinion) Much of an evening is walking known ground a step at a time.
- **Candidates.** Following a route over known ground, stopping at anything new, which the specification already
  requires (G-WLD-007). No rule changes.

### C8. The rules are hidden

- **So what.** (Opinion) Skill, morale and control decide battles and none is explained in play; B1 shows one place
  where what is shown is wrong.
- **Candidates.** What the specification already requires: the cost before committing, the sum behind a result
  (G-UX-002, G-CMB-005). No rule changes.

## Three kinds of change

| Kind | What | Where it lives | Examples |
|---|---|---|---|
| Telling | The player is shown what the rules already do | Every profile, the reference included | B1a, B2a, C3a, C4a, C7, C8 |
| Rule | The rules do something else | A variant profile beside `reference`, never in it | B1b, B2b, B4, B5, C1, C3b, C4b, C4c, C5 |
| Play | The opponent decides differently | A named policy, chosen apart from the rules | B6 |

Telling comes first: it is the cheapest, cannot unbalance anything, and several gaps may turn out to need no more.

A rule change goes through the same door every time: say what it is for; add it as one switch of a variant profile, the
reference untouched; run both suites and the study's measurements before and after; play it; the maintainer decides. A
variant gets its own baseline and is never compared with the reference's by accident.

## When

| When | What can be done | Why not sooner |
|---|---|---|
| Now | B4, B5: one switch each in a variant profile, measured against the tables above. B1a and B2a on the battle screen. | Nothing missing. |
| With the expedition (M2) | C1, C3, C5, with the expedition pilot. C7, C8 as the adventure screens are built. | They need a clock, a purse and a map. |
| With the whole catalogue (importer) | B3: the value of every troop, then prices. B1b, once all five morale groups can meet. | Three troops say too little. |
| With a tactical opponent | B6, and B3 again with shooters played properly. | The plain pilot cannot show it. |
| Before any release called faithful | One experiment in the DOS game on B1: which morale rule it really uses. | It decides whether B1 is the original's or the reference's. |

## Decided by the maintainer, 13 November 2026

1. A variant profile is started now, before the expedition.
2. No kind of change is ruled in or out beforehand: what is measured and played to be better is adopted, whichever it
   is. The reference rules stay as they are only because everything is measured against them.
3. None of the gaps listed is out of bounds.

## The variant: `modern-v0.1`

`content/modern/rules.json` is the reference rules with one switch for each rule change that can be tried with three
troops and a battle. Same troops, same chart, same battles; a test holds the content equal to the reference's. Each
switch stands alone, so each can be measured alone; the file has all four on.

| Gap | Switch | Reference | Variant |
|---|---|---|---|
| B4 | the count that answers a blow | the number before the blow | the survivors |
| B5 | the roll of a blow | one roll | the mean of two rolls |
| B1 | the morale rule | the one the reference attributes to the original | the one the army screen shows |
| B2 | a stack over the leadership | out of control, an enemy to all | in control; only as many fight as the leadership covers |

The first three are choices the reference or the prototype already describe. The fourth is this project's own design:
the reference offers nothing between obedience and mutiny.

What each does to the tables above is still to be measured; until then none of them is adopted, and the variant's name
promises nothing about which will stay.

Not in the variant, with the reason: prices (B3) need the whole catalogue; the opponent's play (B6) is a policy, not a
rule; every campaign point needs a campaign.
