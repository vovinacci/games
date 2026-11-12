# Balance regression: small, from the first battle

This replaces the v0.2 balance chapter. Early measurement remains mandatory; a separate benchmark framework is not. Old
B-REG-007 through B-REG-020 are retired, not additional requirements to implement.

**B-REG-001 - Start with gameplay.** In the same increment that first resolves a headless battle, run a fixed set of
scenarios/seeds, persist outcomes and repeat one run to check determinism. Before a battle exists, ordinary
rule/serialization tests suffice. No M0 fake-report comparator milestone.

**B-REG-002 - Keep the experiment identifiable.** Record engine revision, rule and content IDs/hashes, scenario IDs,
pilot versions and the explicit seed list. Freeze unrelated inputs when comparing a gameplay change. For a rules change
keep pilots fixed; for an AI change keep rules/data fixed. Changed seed lists or scenario cohorts require an explicit
new baseline, not an unexplained percentage change. Matching initial seeds does not guarantee matching later random
events when the changed program consumes random draws differently.

**B-REG-003 - Record a few useful results.** Per case/seed: outcome, per-troop survivors, rounds/actions, consumables
and error/stall status. Derive casualties and replacement cost using a frozen evaluation price table. If prices change,
show the changed native cost too. At M2 add days, spending, rewards and final gold. Keep each case visible; do not
conceal a changed encounter in an aggregate score.

**B-REG-004 - Compare, then review.** Pure refactors must preserve the fixed corpus's gameplay results exactly. Intended
rule/AI changes produce a short diff and a note stating expected versus unexpected effects. Review material changes
before replacing the baseline. Git history plus the previous report is sufficient; there is no approval command,
signature system or automated rebalancer.

**B-REG-005 - Fail real defects.** Missing/duplicate runs, illegal commands, crashes, non-deterministic repeats and
unexpected stalls fail a required check. An intentionally unreachable scenario is a diagnostic and never a normal
defeat. A missing baseline is reported as unrecorded and is not a passing comparison. An expected balance change needs
review, not an invented universal win-rate or confidence threshold. A small corpus shows what changed in that corpus,
not a population-level claim about difficulty.

**B-REG-006 - Grow with the game.** Keep synthetic fixtures as engine regression coverage. Start a KB reference-profile
baseline before tuning KB AI; synthetic win rates do not transfer. Add an expedition/economy case with those features,
then held-out scenarios when tuning becomes substantial. Deliberately change a known-sensitive damage or retaliation
rule once to confirm the report detects it. No dashboard, scheduled tournament, performance farm or statistical gate is
needed for the first milestone.

## Included starter suite

`balance/suite.json` contains four synthetic cases with eight explicit seeds each. That is 32 planned engineering runs,
not a sample-size adequacy claim. Activate cases as their mechanics exist; partial coverage is visible. The unreachable
case expects the prototype's round budget diagnostic. No new gameplay numbers are added. `baselines/bootstrap.json` is
honestly unrecorded. These files have schema version 2; do not consume old schema-1 reports without an explicit
conversion.

## Eventual tool interface (not implemented in this pack)

From repository root:

```sh
cargo run -p bounty-tools -- balance run --suite bounty/balance/suite.json --out /tmp/bounty-current.json
cargo run -p bounty-tools -- balance compare --baseline <saved-report.json> --candidate /tmp/bounty-current.json
```

`run` writes raw rows plus a small per-case summary. `compare` produces a human readable diff. Initially one process and
canonical case/seed order are sufficient. The implementation may instead use an existing repository test harness; these
commands do not mandate a new generic CLI architecture.

Before baseline acceptance, record both files and a short change note. CI checks validity and exact preservation where
claimed. For intentional gameplay shifts, require the attached report and maintainer review; do not automatically
replace expectations to turn the job green.
