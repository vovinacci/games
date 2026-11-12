# 01. Vision and scope

## Product statement

**D-VIS-001.** Build a compact, single-player fantasy strategy adventure that preserves the original King's Bounty's
identity while providing readable controls, reliable saves, and opponents worth learning. The intended reward is
enjoyable programming and enjoyable play, not commercial growth or content volume.

The core loop for this project is: explore, assess an encounter, recruit or reorganise, fight, spend the reward, advance
the search, and decide whether the next expedition is worth its cost. The campaign must make tactical casualties matter
without making every setback a forced restart.

The principal engineering attraction is the interaction between tactics and campaign balance. Better AI is an explicit
experiment, not a reason to change every original subsystem.

## Proposed defaults

| ID | Decision | Rationale |
|---|---|---|
| D-SCP-001 | Record the DOS source/edition status in M0; pin the actual edition before claiming support. | Avoid combining incompatible ports and later games. |
| D-SCP-002 | Implement in Rust; use Macroquad only in the presentation application. | Keep the existing stack while enabling headless tests. |
| D-SCP-003 | Desktop first: Linux, Windows and macOS; browser as a separately tested follow-on. | Prevent browser storage and input work from blocking the first playable build. |
| D-SCP-004 | One campaign protagonist; no new rival-kingdom simulation. | Preserve scope rather than drifting into HoMM or a 4X game. |
| D-SCP-005 | Classic mechanics and AI policy are separate configuration axes. | Stronger tactics can change difficulty without rewriting rules. |
| D-SCP-006 | Use original or explicitly licensed presentation assets in public builds. | Keep content provenance auditable. |
| D-SCP-007 | English and Ukrainian UI strings from the first screen, both kept complete by a key test. | Decided by the maintainer; the practice already used in Scorch [S5], and `gamekit` supplies it. |
| D-SCP-008 | No multiplayer, online account, analytics service or backend. | A self-contained hobby game is the target. |
| D-SCP-009 | No original binary-save compatibility in version 1. | Implement reliable native saves before import compatibility. |
| D-SCP-010 | Treat all design defaults in this draft as revisable, with a recorded decision. | A first draft is not a historical claim. |
| D-SCP-011 | Plan balance checks from the start; implement result recording with the first complete battle and require it for gameplay changes. | User-confirmed requirement; detect unintended gameplay changes early. |

Macroquad documents desktop and browser support; the staged platform plan above is a project recommendation, not a claim
that every target has already been tested. [S6]

## Rule and policy profiles

The first implementation profile is **Prototype v0.1**, which uses the synthetic data supplied with this pack. Its UI
and saves must visibly identify it as a prototype.

Use **Reference (provisional)** for source-backed mechanics not independently observed in DOS. A verified **Classic**
label requires the appropriate evidence in the rules register; it is not a prerequisite for releasing a clearly labelled
provisional engine. It must not silently load prototype defaults when an original value is missing. Missing required
Classic data is a content-validation failure.

AI selection is independent. Start with the simple development baseline; add a tactical policy only after the
reference-rule subset and its baseline exist. A later **Reference AI** label requires observed reference behaviour; a
rough old-fashioned heuristic must not be advertised as the original AI. Offer enhanced policies as explicit
alternatives. Keep the selected policy in the save and benchmark metadata.

Do not add a dozen small compatibility toggles. Initially permit only whole, versioned rule profiles and separately
versioned AI policies. A balance experiment belongs in a new experimental profile until accepted.

## Delivery boundaries (supersedes v0.2)

M0: integrate with the parent workspace, select the source/content approach and record the narrow rule subset. No
separate benchmark platform is required.

M1a: one headless synthetic battle with exact rule tests and recorded outcomes. These fixtures validate engineering, not
KB balance or fun.

M1b: a small reference-rule battle subset, source-backed troop values, skill, morale, control/leadership and documented
activation semantics. Record a baseline and a procedural playable view before tuning advanced AI. Provisional adoption
from a pinned reference is allowed; no unverified "exact DOS" label.

M2: a complete small expedition under the chosen reference policy, plus the supplied synthetic expedition for exact
integration tests. One safe save/restore flow works; battle losses have observable campaign costs.

M3: required campaign systems and supported local-data import; every field used by implemented content has known
semantics or an explicitly adopted deviation.

M4: complete labelled campaign and broader balance review. This is not gated on all historical details being
independently rediscovered. An unresolved issue must be accepted as provisional/deviated, scoped out, or genuinely block
an advertised capability; it may not silently acquire a false "verified" label.

M5: public engine/demo release, imported-data support only for tested editions, clear limitations, approved distribution
manifest and documented save policy.

## Content and presentation choice

Proposed route: an engine that optionally loads data from a user-selected local DOS copy, plus a tiny self-authored
procedural demo. This is not a generic gamepack platform. One adapter and one internal content model are enough. It is
not a commitment to recreate every original sprite by hand. See CONTENT-AND-ASSETS.md.

## Non-goals for version 1

No 3D engine, hex-grid conversion, castle-building economy, diplomacy system, multiplayer, user scripting,
general-purpose engine, workshop service, extensive editor, random terrain generator, or mobile-specific interface. No
large original story campaign is required. User-facing key remapping, multiple save slots, rotating autosave, selectable
sprite filters and historical-save migrations are not MVP gates; see the roadmap for the safe minimum.

A data validator and a small developer scenario loader are in scope. A polished map editor is not. Fixed authored
scenarios should exist before procedural experiments.

## Feel targets and evaluation

The player should understand what an action costs before committing, understand why a battle outcome occurred, and be
able to stop and resume without losing a meaningful decision. Travel must not become an exercise in hundreds of
uninformative clicks. Automating known travel must not reveal unknown information or skip decisions.

Use playtests to examine: whether an early battle teaches a rule, whether a costly victory changes the next expedition,
whether retreat/recovery remains interesting, and whether the final search is understandable. These are qualitative
targets; no arbitrary retention percentage is a release requirement.

Preserve discoveries. Tooltips should explain rules, not reveal hidden campaign answers. Fast animations and better
controls must not become disguised free information.
