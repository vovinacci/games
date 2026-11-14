//! The reference rules file: rules and numbers taken from a reimplementation of the original game
//! (`docs/spec/07-RULES-REGISTER.md` says which, at which revision, and what is known to be doubtful). Read as
//! strictly as the prototype file: every switch must name the behaviour the engine has, and what this subset does
//! not cover (spells, special abilities) is refused, not approximated.

use serde::Deserialize;

use super::{
    Ability, Board, ContentError, MODERN_RULESET, Profile, REFERENCE_RULESET, Rules, Troop,
    check_board, invalid, only, read, unique,
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
    /// Where a variant of these rules departs from the reference. All off in the reference itself.
    pub switches: Switches,
}

/// What happens in a week whose upkeep the purse cannot cover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Unpaid {
    /// The purse is emptied and every stack stays. The reference's rule.
    #[default]
    Stay,
    /// Stacks are paid in slot order, each in full or not at all; a stack not paid leaves.
    Leave,
}

/// How many creatures of a stack answer a blow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Answer {
    /// As many as the stack had before the blow: creatures killed by it still strike. The reference's rule.
    #[default]
    Before,
    /// Those left after the blow.
    Survivors,
    /// The mean of the two, rounded down: half of those the blow killed still strike.
    Mean,
}

/// The rules a variant may set differently from the reference: one switch for each gap the study
/// (`docs/spec/10-GAMEPLAY-GAPS.md`) proposes a rule change for. Each stands alone, so each can be measured alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Switches {
    /// B4. How many creatures answer a blow.
    pub answer: Answer,
    /// B5. A blow takes the mean of two rolls instead of one: the same average, less swing.
    pub two_rolls: bool,
    /// B1. Morale by the rule the army screen shows: a stack alone is High; every other stack of the side is
    /// looked up as (self, other); any Low gives Low, all High gives High, else Normal.
    pub army_view_morale: bool,
    /// B2. A stack too large for the leadership does not turn on its side; only as many of its creatures fight as
    /// the leadership covers.
    pub leadership_caps_fighters: bool,
    /// C1. What a week does whose upkeep the purse cannot cover.
    pub unpaid: Unpaid,
    /// B4, cured after the battle: of each stack's creatures that fell in a battle the hero won, this many in a
    /// hundred return, rounded down. 0 in the reference.
    pub fallen_return_percent: u32,
}

/// Morale groups, A to E.
const GROUPS: usize = 5;
const GROUP_NAMES: [&str; GROUPS] = ["A", "B", "C", "D", "E"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema_version: u32,
    #[serde(default)]
    campaign: Option<Campaign>,
    id: String,
    #[serde(rename = "status")]
    _status: String,
    /// The ruleset whose battle files this one plays: a variant names the reference here.
    #[serde(default)]
    plays_battles_of: Option<String>,
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
    /// Absent in the reference file, which rolls once.
    #[serde(default)]
    rolls: Option<String>,
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
    /// Absent in the reference file, where a stack over the leadership is out of control.
    #[serde(default)]
    over_leadership: Option<String>,
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
    #[serde(default)]
    abilities: Vec<String>,
    morale_group: String,
    recruit_cost: u32,
}

/// The rules of the campaign a variant may change. Absent, they are the reference's.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Campaign {
    unpaid_week: String,
    fallen_of_a_won_battle_return_percent: u32,
}

fn check_switches(file: &File) -> Result<(), ContentError> {
    only("schema_version", &file.schema_version, &1)?;
    if file.id != REFERENCE_RULESET && file.id != MODERN_RULESET {
        return Err(ContentError::Unsupported(format!(
            "id is {:?}; only {REFERENCE_RULESET:?} and {MODERN_RULESET:?} are read here",
            file.id
        )));
    }
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

/// Reads one switch: the reference's value, or the variant's where the ruleset is a variant. Anything else, and
/// the variant's value in the reference itself, is refused.
fn switch(
    what: &str,
    found: &str,
    reference: &str,
    variant: &str,
    may_vary: bool,
) -> Result<bool, ContentError> {
    if found == reference {
        Ok(false)
    } else if found == variant && may_vary {
        Ok(true)
    } else {
        let allowed = if may_vary {
            format!("{reference:?} or {variant:?}")
        } else {
            format!("{reference:?}")
        };
        Err(ContentError::Unsupported(format!(
            "{what} is {found:?}; this ruleset allows {allowed}"
        )))
    }
}

/// The switches of a file. The reference must have every one at the reference's value; `modern` may set each.
fn switches(file: &File) -> Result<Switches, ContentError> {
    let vary = file.id == MODERN_RULESET;
    let damage = &file.damage;
    let answer = match damage.count_used.as_str() {
        "count_before_the_exchange" => Answer::Before,
        "count_at_the_blow" if vary => Answer::Survivors,
        "mean_of_before_and_after_the_blow" if vary => Answer::Mean,
        other => {
            return Err(ContentError::Unsupported(format!(
                "damage.count_used is {other:?}, which this ruleset does not allow"
            )));
        }
    };
    // The older yes-or-no line must agree with it.
    only(
        "damage.retaliation_uses_survivors",
        &damage.retaliation_uses_survivors,
        &(answer == Answer::Survivors),
    )?;
    let rolls = damage.rolls.as_deref().unwrap_or("one");
    let control = file
        .control
        .over_leadership
        .as_deref()
        .unwrap_or("out_of_control");
    let (unpaid, fallen) =
        file.campaign
            .as_ref()
            .map_or(("purse_emptied_every_stack_stays", 0), |campaign| {
                (
                    campaign.unpaid_week.as_str(),
                    campaign.fallen_of_a_won_battle_return_percent,
                )
            });
    if fallen > 100 || (fallen != 0 && !vary) {
        return Err(ContentError::Unsupported(format!(
            "campaign.fallen_of_a_won_battle_return_percent is {fallen}, which this ruleset does not allow"
        )));
    }
    Ok(Switches {
        unpaid: if switch(
            "campaign.unpaid_week",
            unpaid,
            "purse_emptied_every_stack_stays",
            "stacks_paid_in_slot_order_unpaid_leave",
            vary,
        )? {
            Unpaid::Leave
        } else {
            Unpaid::Stay
        },
        fallen_return_percent: fallen,
        answer,
        two_rolls: switch("damage.rolls", rolls, "one", "mean_of_two", vary)?,
        army_view_morale: switch(
            "morale.rule",
            &file.morale.rule,
            "lowest_rank_over_own_side_self_included_chart_read_other_then_self",
            "alone_high_else_others_read_self_then_other_any_low_is_low_all_high_is_high",
            vary,
        )?,
        leadership_caps_fighters: switch(
            "control.over_leadership",
            control,
            "out_of_control",
            "only_what_leadership_covers_fights",
            vary,
        )?,
    })
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
    // Movement of at least 1: a stack acts while it has movement left, so one without any could neither strike
    // nor use its flights. No troop of the original has less.
    if from.skill == 0 || from.hp == 0 || from.movement == 0 {
        return invalid(format!(
            "troop {id:?} needs skill, health and movement of at least 1"
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
    let mut abilities = Vec::new();
    for name in &from.abilities {
        let Some(ability) = Ability::ALL.into_iter().find(|a| a.name() == name) else {
            // A troop is refused, not fielded without an ability its file gives it.
            return invalid(format!(
                "troop {id:?} has the ability {name:?}, which these rules do not have"
            ));
        };
        if abilities.contains(&ability) {
            return invalid(format!("troop {id:?} names the ability {name:?} twice"));
        }
        abilities.push(ability);
    }
    // The reference keeps a magic shot as one number; a range would ask for a roll it never makes.
    if abilities.contains(&Ability::MagicShot)
        && !matches!((from.ranged_min, from.ranged_max), (Some(min), Some(max)) if min == max)
    {
        return invalid(format!(
            "troop {id:?} has a magic shot, which is one number: ranged_min and ranged_max the same"
        ));
    }
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
        abilities,
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
    let switches = switches(&file)?;
    // The reference plays its own battles; a variant plays those of the ruleset it names, which must be the
    // reference: there are no others.
    let battles_id = file
        .plays_battles_of
        .clone()
        .unwrap_or_else(|| file.id.clone());
    if battles_id != file.id && (file.id != MODERN_RULESET || battles_id != REFERENCE_RULESET) {
        return invalid(format!(
            "{} cannot play the battles of {battles_id:?}",
            file.id
        ));
    }
    unique("troop", file.troops.iter().map(|t| t.id.as_str()))?;
    let flights = file.flight.flights_per_activation;
    let troops = file
        .troops
        .into_iter()
        .map(|t| troop(t, flights))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Rules {
        id: file.id,
        battles_id,
        profile: Profile::Reference(Reference {
            morale_chart,
            switches,
        }),
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

    // The whole table of the original's troops.
    #[test]
    fn the_reference_file_has_all_twenty_five_troops() {
        let rules = Rules::from_json(REFERENCE).unwrap();
        assert_eq!(rules.troops.len(), 25);
        let ids = |keep: &dyn Fn(&Troop) -> bool| -> Vec<&str> {
            let kept = rules.troops.iter().filter(|t| keep(t));
            kept.map(|t| t.id.as_str()).collect()
        };
        assert_eq!(
            ids(&|t| t.ranged().is_some()),
            ["orcs", "archers", "elves", "druids", "archmages", "giants"]
        );
        assert_eq!(
            ids(&|t| t.flights_per_activation > 0),
            ["sprites", "archmages", "vampires", "demons", "dragons"]
        );
        assert_eq!(
            ids(&|t| t.has(Ability::Undead)),
            ["skeletons", "zombies", "ghosts", "vampires"]
        );
        let groups: std::collections::BTreeSet<usize> =
            rules.troops.iter().filter_map(|t| t.morale_group).collect();
        assert_eq!(
            groups.into_iter().collect::<Vec<_>>(),
            [0, 1, 2, 3, 4],
            "A to E"
        );
    }

    // A troop is never fielded without an ability its file gives it: one these rules do not have is refused.
    #[test]
    fn an_ability_the_rules_do_not_have_is_refused() {
        for unknown in ["breath", "Regeneration", "fly"] {
            let error = with(r#""regeneration""#, &format!("{unknown:?}")).unwrap_err();
            assert!(error.to_string().contains(unknown), "{error}");
        }
        assert!(with(r#""regeneration""#, r#""regeneration", "regeneration""#).is_err());
    }

    // A magic shot is one number: with a range, or on a troop that does not shoot, the file is refused.
    #[test]
    fn a_magic_shot_needs_one_number_to_shoot_with() {
        assert!(with(r#""regeneration""#, r#""magic_shot""#).is_err());
        let error = with(
            r#""ranged_min": 10,
      "ranged_max": 10,"#,
            r#""ranged_min": 10,
      "ranged_max": 12,"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("magic shot"), "{error}");
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
            is_invalid(with(r#""movement": 3"#, r#""movement": 0"#)),
            "no movement"
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
