# 08. Sources, attribution and evidence limits

Research date: **12 November 2026**. These references inform the draft; they are not copied into the distribution. Most
of this pack is an original proposed engineering specification, not a paraphrase of an existing game document.

## S1 - Original New World Computing manual

Title: King's Bounty, First Edition, copyright 1990 New World Computing.

Source URL: `https://www.bestoldgames.net/download/games/kings-bounty/kings-bounty-manual.pdf`

Relevant printed sections: Starting the Adventure (23 onward); Adventuring (28 onward); Combat (37-40); Puzzle and
winning (41); appendices for spells, creatures, artefacts and locations.

Authority: primary documentation for the original game. The web tool exposed parsed text; attempts to obtain page
screenshots failed because of the source size. No numeric table was visually transcribed for this pack. Documentation is
not proof that the executable behaves identically at every boundary.

## S2 - OpenBounty project

Source URL: `https://github.com/dannyheskett/openbounty`

Authority: the maintainers' description of their own reimplementation, not original DOS behaviour. Useful for
identifying a contemporary reference project and its available engineering documentation. This project is not a
dependency of the proposed Rust remake.

## S3 - OpenBounty reproduction specification

Source URL: `https://raw.githubusercontent.com/dannyheskett/openbounty/main/docs/OPENBOUNTY-SPEC.md`

Relevant sections: rule/data inventory; combat; known deviations, especially section 38. The document explicitly
distinguishes some behaviours from DOS and OpenKB. It is therefore a research lead, not an oracle. Its implementation
details should not be imported wholesale into a new Rust design.

## S4 - OpenBounty combat definitions and implementation

Header URL: `https://raw.githubusercontent.com/dannyheskett/openbounty/main/engine/include/combat.h`

Implementation URL: `https://raw.githubusercontent.com/dannyheskett/openbounty/main/engine/combat.c`

Used for board-dimension corroboration and examples of rule details needing comparison. Source comments and
implementation behaviour can themselves disagree. Reproduction of an existing reimplementation is not necessarily
reproduction of the DOS original.

## S5 - The user's existing Scorch project

Repository URL: `https://github.com/vovinacci/scorch-game/`

Conventions URL: `https://raw.githubusercontent.com/vovinacci/scorch-game/main/AGENTS.md`

Used narrowly to suggest continuity with English/Ukrainian localisation, separate player/developer documentation,
lint/test discipline and reproducible screenshot hooks. This draft does not claim to have performed a full code audit or
identified reusable implementations.

## S6 - Macroquad official site

Source URL: `https://macroquad.rs/`

Used for the documented platform capabilities. Exact dependency versions and runtime behaviour must be validated in the
project; this draft does not prescribe an untested dependency update.

## S7 - OpenBounty content catalogue

Source URL: `https://raw.githubusercontent.com/dannyheskett/openbounty/main/assets/kings-bounty/game.json`

Used to corroborate the scope inventory and identify data categories. Values in this catalogue are reimplementation
evidence, not automatically accepted Classic values. The full catalogue, maps and assets are not included in this pack.

## Evidence hierarchy

For a faithfully labelled rule: select the DOS executable and record its hash/version; observe a reproducible input
sequence; compare its visible result with the manual; use reimplementations to identify likely explanations. Record
disagreements, rather than automatically promoting whichever source is easiest to read.

The URLs above point to branches and can change. What the project relies on is pinned: OpenBounty [S2, S3, S4, S7] at
revision `aace551ca625e0c723adff0cb13b22ca2f806dee` (MIT licence, read on 12 November 2026), as recorded in
`07-RULES-REGISTER.md`. A later revision is used only after the register is updated with what changed.

## Provenance and publication policy

Track origin and permission separately for source code, graphics, audio, fonts, maps and text. A repository's code
license is not a substitute for checking the provenance of every included asset. Use self-authored test content while
that review is incomplete. This is a conservative project policy, not a legal opinion about a particular distribution.

No copyrighted game executable, manual, commercial asset or font file is included with this specification pack.

## v0.3 review references (checked 12 November 2026)

S8 - OpenKB project-maintained documentation: `https://sourceforge.net/p/openkb/wiki/Home/`
`https://sourceforge.net/p/openkb/wiki/Formats/` The project describes original-file reading and documents DOS format
categories. Useful for adapter planning, not executable parity or rights clearance.

S9 - OpenBounty project README: `https://raw.githubusercontent.com/dannyheskett/openbounty/main/README.md` Documents
--extract and port-authored extractor constants. No assumption that all project distribution statements are mutually
consistent or legally vetted.

S10 - DOSBox-X upstream debugger guide:
`https://raw.githubusercontent.com/joncampbell123/dosbox-x/master/README.debugger` Used for debugger availability,
bounded memory dumps and breakpoint caveats. Use the selected build's HELP; no universal KB memory addresses are
supplied.

S11 - OpenBounty specification, section 25.3:
`https://raw.githubusercontent.com/dannyheskett/openbounty/main/docs/OPENBOUNTY-SPEC.md` Documents player-side-first,
slot-order activation. This corrects an unexamined speed-initiative assumption; it is not independent observation of a
DOS executable.

S12 - Cargo documentation: `https://doc.rust-lang.org/cargo/reference/workspaces.html`
`https://doc.rust-lang.org/cargo/reference/manifest.html#the-publish-field` Used for parent-workspace integration and
the distinction between publish=false and executable release packaging.

S13 - U.S. Copyright Office, Games and FAQ: `https://www.copyright.gov/register/tx-games.html`
`https://www.copyright.gov/help/faq/faq-protect.html` General U.S. distinction between methods/ideas/names and protected
expression; not a legal conclusion for this user's jurisdiction or project.

S14 - USPTO, trademark confusion and clearance: `https://www.uspto.gov/trademarks/search/likelihood-confusion`
`https://www.uspto.gov/trademarks/search/comprehensive-clearance-search-similar-trademarks` No search of the actual
proposed brand has been completed; no King's Bounty registration's present status, scope or ownership is asserted.

All URLs here are retrieval references, not immutable implementation pins. A full OpenBounty commit could not be
resolved in this environment; no hash is invented. Before adopting a rule in code, the developer should record the
actual checked-out revision or a retained local source snapshot hash. Reading two descendant projects does not give two
independent historical confirmations.
