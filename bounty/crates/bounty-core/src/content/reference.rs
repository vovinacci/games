//! The reference rules file: rules and numbers taken from a reimplementation of the original game
//! (`docs/spec/07-RULES-REGISTER.md` says which, at which revision, and what is known to be doubtful). Read as
//! strictly as the prototype file: every switch must name the behaviour the engine has, and what this subset does
//! not cover (spells, special abilities) is refused, not approximated.

use serde::Deserialize;

use super::{
    Board, ContentError, Profile, REFERENCE_RULESET, Rules, Troop, check_board, invalid, only,
    read, unique,
};

/// What a stack's morale does to the damage it deals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Morale {
    Normal,
    /// Halves the damage.
    Low,
    /// Adds half again.
    High,
}

/// What the reference rules have beyond the prototype's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    /// `chart[a][b]`: the entry in the row of group `a` and the column of group `b`. Which of the two is the
    /// troop whose morale is asked is the rule's business, not the chart's.
    pub morale_chart: [[Morale; GROUPS]; GROUPS],
}

/// Morale groups, A to E.
const GROUPS: usize = 5;
const GROUP_NAMES: [&str; GROUPS] = ["A", "B", "C", "D", "E"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema_version: u32,
    id: String,
    #[serde(rename = "status")]
    _status: String,
    provenance: Provenance,
    board: Board,
    rng: Rng,
    turns: Turns,
    movement: Movement,
    flight: Flight,
    damage: Damage,
    morale: MoraleRule,
    control: Control,
    round_limit: u32,
    round_limit_outcome: String,
    hero_magic: HeroMagic,
    troops: Vec<FileTroop>,
    spells: Vec<serde_json::Value>,
}

/// Where the values come from. Required, so a reference file cannot lose its source.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    source: String,
    revision: String,
    files: Vec<String>,
    #[serde(rename = "note")]
    _note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rng {
    algorithm: String,
    multiplier: u64,
    increment: u64,
    bits: u32,
    sampling: String,
    constant_range_consumes_draw: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Turns {
    order: String,
    flags_reset: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Movement {
    neighbours: u32,
    step_cost: u32,
    diagonal_requires_clear_orthogonals: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Flight {
    flights_per_activation: u32,
    costs_movement: bool,
    last_flight_ends_activation: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Damage {
    method: String,
    skill_term: String,
    count_used: String,
    retaliation_uses_survivors: bool,
    injury_persists_during_battle: bool,
    injury_heals_after_battle: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MoraleRule {
    applies_to: String,
    rule: String,
    rank_order: Vec<String>,
    low: String,
    high: String,
    groups: Vec<String>,
    chart: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    out_of_control_when: String,
    checked: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeroMagic {
    player_enabled: bool,
    enemy_enabled: bool,
    casts_per_round: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileTroop {
    id: String,
    name: String,
    skill: u32,
    hp: u32,
    movement: u32,
    melee_min: u32,
    melee_max: u32,
    #[serde(default)]
    ranged_min: Option<u32>,
    #[serde(default)]
    ranged_max: Option<u32>,
    shots: u32,
    flies: bool,
    morale_group: String,
    recruit_cost: u32,
}

fn check_switches(file: &File) -> Result<(), ContentError> {
    only("schema_version", &file.schema_version, &1)?;
    only("id", &file.id.as_str(), &REFERENCE_RULESET)?;
    only(
        "rng.algorithm",
        &file.rng.algorithm.as_str(),
        &"reference-lcg64-v1",
    )?;
    only("rng.multiplier", &file.rng.multiplier, &25_214_903_917)?;
    only("rng.increment", &file.rng.increment, &11)?;
    only("rng.bits", &file.rng.bits, &64)?;
    only(
        "rng.sampling",
        &file.rng.sampling.as_str(),
        &"high_32_bits_modulo_span",
    )?;
    only(
        "rng.constant_range_consumes_draw",
        &file.rng.constant_range_consumes_draw,
        &false,
    )?;
    only(
        "turns.order",
        &file.turns.order.as_str(),
        &"player_side_then_enemy_side_by_slot",
    )?;
    only(
        "turns.flags_reset",
        &file.turns.flags_reset.as_str(),
        &"before_each_side",
    )?;
    only("movement.neighbours", &file.movement.neighbours, &8)?;
    only("movement.step_cost", &file.movement.step_cost, &1)?;
    let corners = file.movement.diagonal_requires_clear_orthogonals;
    only(
        "movement.diagonal_requires_clear_orthogonals",
        &corners,
        &false,
    )?;
    only(
        "flight.flights_per_activation",
        &file.flight.flights_per_activation,
        &2,
    )?;
    only("flight.costs_movement", &file.flight.costs_movement, &false)?;
    only(
        "flight.last_flight_ends_activation",
        &file.flight.last_flight_ends_activation,
        &true,
    )?;
    only(
        "damage.method",
        &file.damage.method.as_str(),
        &"roll_times_count_times_skill_term_over_ten",
    )?;
    only(
        "damage.skill_term",
        &file.damage.skill_term.as_str(),
        &"attacker_skill_plus_5_minus_target_skill",
    )?;
    only(
        "damage.count_used",
        &file.damage.count_used.as_str(),
        &"count_before_the_exchange",
    )?;
    only(
        "damage.retaliation_uses_survivors",
        &file.damage.retaliation_uses_survivors,
        &false,
    )?;
    only(
        "damage.injury_persists_during_battle",
        &file.damage.injury_persists_during_battle,
        &true,
    )?;
    only(
        "damage.injury_heals_after_battle",
        &file.damage.injury_heals_after_battle,
        &true,
    )?;
    check_morale_control_and_magic(file)
}

fn check_morale_control_and_magic(file: &File) -> Result<(), ContentError> {
    let morale = &file.morale;
    only(
        "morale.applies_to",
        &morale.applies_to.as_str(),
        &"side_with_a_hero_and_stack_under_control",
    )?;
    let rule = "lowest_rank_over_own_side_self_included_chart_read_other_then_self";
    only("morale.rule", &morale.rule.as_str(), &rule)?;
    let ranks: Vec<&str> = morale.rank_order.iter().map(String::as_str).collect();
    only("morale.rank_order", &ranks, &vec!["normal", "low", "high"])?;
    only("morale.low", &morale.low.as_str(), &"halve")?;
    only("morale.high", &morale.high.as_str(), &"add_half")?;
    let groups: Vec<&str> = morale.groups.iter().map(String::as_str).collect();
    only("morale.groups", &groups, &GROUP_NAMES.to_vec())?;
    let when = "health_each_times_count_exceeds_leadership";
    only(
        "control.out_of_control_when",
        &file.control.out_of_control_when.as_str(),
        &when,
    )?;
    only(
        "control.checked",
        &file.control.checked.as_str(),
        &"before_every_activation",
    )?;
    only(
        "round_limit_outcome",
        &file.round_limit_outcome.as_str(),
        &"budget_exceeded",
    )?;
    // This subset has no hero spells and no special abilities; a file that asks for them is refused.
    only(
        "hero_magic.player_enabled",
        &file.hero_magic.player_enabled,
        &false,
    )?;
    only(
        "hero_magic.enemy_enabled",
        &file.hero_magic.enemy_enabled,
        &false,
    )?;
    only(
        "hero_magic.casts_per_round",
        &file.hero_magic.casts_per_round,
        &0,
    )?;
    only("spells", &file.spells.len(), &0)?;
    Ok(())
}

fn chart(rows: &[String]) -> Result<[[Morale; GROUPS]; GROUPS], ContentError> {
    let mut chart = [[Morale::Normal; GROUPS]; GROUPS];
    if rows.len() != GROUPS {
        return invalid(format!(
            "morale chart has {} rows, not {GROUPS}",
            rows.len()
        ));
    }
    for (row, text) in rows.iter().enumerate() {
        if text.chars().count() != GROUPS {
            return invalid(format!(
                "morale chart row {text:?} does not have {GROUPS} entries"
            ));
        }
        for (column, entry) in text.chars().enumerate() {
            chart[row][column] = match entry {
                'N' => Morale::Normal,
                'L' => Morale::Low,
                'H' => Morale::High,
                other => return invalid(format!("morale chart entry {other:?} is not N, L or H")),
            };
        }
    }
    Ok(chart)
}

fn troop(from: FileTroop, flights: u32) -> Result<Troop, ContentError> {
    let id = &from.id;
    if from.skill == 0 || from.hp == 0 {
        return invalid(format!(
            "troop {id:?} needs a skill and health of at least 1"
        ));
    }
    if from.melee_min > from.melee_max {
        return invalid(format!("troop {id:?} has melee_min above melee_max"));
    }
    match (from.ranged_min, from.ranged_max) {
        (None, None) if from.shots == 0 => {}
        (Some(min), Some(max)) if min <= max && from.shots > 0 => {}
        _ => {
            return invalid(format!(
                "troop {id:?}: ranged_min, ranged_max and shots do not fit together"
            ));
        }
    }
    let Some(group) = GROUP_NAMES
        .iter()
        .position(|name| *name == from.morale_group)
    else {
        return invalid(format!(
            "troop {id:?} has morale group {:?}, not one of A to E",
            from.morale_group
        ));
    };
    Ok(Troop {
        id: from.id,
        name: from.name,
        hp: from.hp,
        movement: from.movement,
        melee_min: from.melee_min,
        melee_max: from.melee_max,
        ranged_min: from.ranged_min,
        ranged_max: from.ranged_max,
        shots: from.shots,
        flights_per_activation: if from.flies { flights } else { 0 },
        recruit_cost: from.recruit_cost,
        skill: from.skill,
        morale_group: Some(group),
    })
}

/// Reads and checks a reference rules file.
pub(super) fn load(text: &str) -> Result<Rules, ContentError> {
    let file: File = read(text)?;
    check_switches(&file)?;
    check_board(&file.board, file.round_limit)?;
    let source = &file.provenance;
    if source.source.is_empty() || source.revision.len() != 40 || source.files.is_empty() {
        return invalid(
            "provenance needs a source, a full 40-character revision and the files read".to_owned(),
        );
    }
    let morale_chart = chart(&file.morale.chart)?;
    unique("troop", file.troops.iter().map(|t| t.id.as_str()))?;
    let flights = file.flight.flights_per_activation;
    let troops = file
        .troops
        .into_iter()
        .map(|t| troop(t, flights))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Rules {
        id: file.id,
        profile: Profile::Reference(Reference { morale_chart }),
        board: file.board,
        round_limit: file.round_limit,
        troops,
        spells: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::RULES as PROTOTYPE;

    const REFERENCE: &str = include_str!("../../../../content/reference/rules.json");

    fn with(old: &str, new: &str) -> Result<Rules, ContentError> {
        assert!(
            REFERENCE.contains(old),
            "{old:?} is in the reference rules file"
        );
        Rules::from_json(&REFERENCE.replacen(old, new, 1))
    }

    #[test]
    fn the_reference_file_loads_with_its_troops_and_chart() {
        let rules = Rules::from_json(REFERENCE).unwrap();
        assert_eq!(rules.id, REFERENCE_RULESET);
        let Profile::Reference(reference) = &rules.profile else {
            panic!("a reference profile")
        };
        let group = |id: &str| rules.troop(id).unwrap().morale_group.unwrap();
        let (sprites, wolves, archers) = (group("sprites"), group("wolves"), group("archers"));
        assert_eq!((sprites, wolves, archers), (2, 3, 1), "groups C, D, B");
        // The entries the first battles turn on.
        assert_eq!(
            reference.morale_chart[wolves][sprites],
            Morale::Low,
            "row D, column C"
        );
        assert_eq!(
            reference.morale_chart[sprites][sprites],
            Morale::High,
            "row C, column C"
        );
        assert_eq!(
            reference.morale_chart[sprites][wolves],
            Morale::Normal,
            "row C, column D"
        );
        assert_eq!(reference.morale_chart[4][1], Morale::Low, "row E, column B");

        let sprites = rules.troop("sprites").unwrap();
        assert_eq!(
            (
                sprites.skill,
                sprites.hp,
                sprites.movement,
                sprites.flights_per_activation
            ),
            (1, 1, 1, 2)
        );
        let archers = rules.troop("archers").unwrap();
        assert_eq!(
            (archers.skill, archers.ranged(), archers.shots),
            (2, Some((1, 3)), 12)
        );
        assert_eq!(rules.troop("wolves").unwrap().flights_per_activation, 0);
        assert_eq!(rules.spells, Vec::new());
    }

    #[test]
    fn the_prototype_file_is_still_the_prototype() {
        let rules = Rules::from_json(PROTOTYPE).unwrap();
        assert_eq!(rules.profile, Profile::Prototype);
        assert!(
            rules
                .troops
                .iter()
                .all(|t| t.skill == 0 && t.morale_group.is_none())
        );
        // A prototype file cannot smuggle in a field of the other profile.
        let smuggled = PROTOTYPE.replacen(r#""hp": 10,"#, r#""hp": 10, "skill": 3,"#, 1);
        assert!(matches!(
            Rules::from_json(&smuggled),
            Err(ContentError::Malformed(_))
        ));
    }

    #[test]
    fn a_broken_reference_file_is_refused() {
        let malformed = |r| matches!(r, Err(ContentError::Malformed(_)));
        let unsupported = |r| matches!(r, Err(ContentError::Unsupported(_)));
        let is_invalid = |r| matches!(r, Err(ContentError::Invalid(_)));
        assert!(
            unsupported(with(
                r#""flights_per_activation": 2"#,
                r#""flights_per_activation": 1"#
            )),
            "other flight"
        );
        assert!(
            unsupported(with(
                r#""flags_reset": "before_each_side""#,
                r#""flags_reset": "each_round""#
            )),
            "turns"
        );
        assert!(
            unsupported(with(r#""low": "halve""#, r#""low": "none""#)),
            "other morale effect"
        );
        assert!(
            unsupported(with(r#""spells": []"#, r#""spells": [{"id": "fireball"}]"#)),
            "spells"
        );
        assert!(
            unsupported(with(
                r#""player_enabled": false"#,
                r#""player_enabled": true"#
            )),
            "hero magic"
        );
        assert!(
            unsupported(with(
                r#""groups": ["A", "B", "C", "D", "E"]"#,
                r#""groups": ["A", "B"]"#
            )),
            "groups"
        );
        assert!(is_invalid(with(r#""NNHNN""#, r#""NNXNN""#)), "chart entry");
        assert!(is_invalid(with(r#""NNHNN","#, "")), "chart row missing");
        assert!(
            is_invalid(with(r#""morale_group": "D""#, r#""morale_group": "F""#)),
            "unknown group"
        );
        assert!(
            is_invalid(with(r#""skill": 1"#, r#""skill": 0"#)),
            "no skill"
        );
        assert!(
            is_invalid(with(r#""id": "wolves""#, r#""id": "sprites""#)),
            "duplicate troop"
        );
        assert!(
            is_invalid(with("aace551ca625e0c723adff0cb13b22ca2f806dee", "aace551")),
            "short revision"
        );
        assert!(malformed(with(r#""skill": 2,"#, "")), "missing skill");
        assert!(
            malformed(with(
                r#""flies": false,"#,
                r#""flies": false, "regenerates": true,"#
            )),
            "unknown ability"
        );
        assert!(
            malformed(with(r#""provenance""#, r#""origin""#)),
            "no provenance"
        );
    }
}
