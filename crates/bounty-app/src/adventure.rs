//! The expedition as the player has it: its state, the log of what happened, and the save kept after every
//! command. Nothing here draws or reads a key, so the tests can play it.

use std::sync::Arc;

use bounty_ai::approach_v1;
use bounty_core::battle;
use bounty_core::campaign::{Action, Command, Expedition, ExpeditionDef, Status};
use bounty_core::content::{Cell, Side};
use gamekit::t;

use crate::saves::Saves;
use crate::words;

pub struct Adventure {
    pub expedition: Expedition,
    /// The cursor on the board of the battle being fought.
    pub cursor: Cell,
    pub log: Vec<String>,
    /// Seconds since the last command, for pacing the enemy.
    pub waited: f32,
    /// The last command could not be saved.
    pub unsaved: bool,
    saves: Saves,
}

impl Adventure {
    /// The saved expedition of `def` if there is one, a new one if not.
    #[must_use]
    pub fn new(def: Arc<ExpeditionDef>, saves: Saves) -> Self {
        match saves.load(&def) {
            Some((expedition, previous)) => {
                let mut adventure = Self::at(expedition, saves);
                adventure.log.push(if previous {
                    t!("log-continued-previous")
                } else {
                    t!("log-continued")
                });
                adventure
            }
            None => Self::at(Expedition::start(def), saves),
        }
    }

    /// This expedition, as it stands.
    #[must_use]
    pub fn at(expedition: Expedition, saves: Saves) -> Self {
        let mut adventure = Self {
            expedition,
            cursor: Cell { x: 0, y: 0 },
            log: Vec::new(),
            waited: 0.0,
            unsaved: false,
            saves,
        };
        adventure.follow_the_turn();
        adventure
    }

    /// Asks the expedition for an action, writes what happened into the log and saves. A refusal is told in the
    /// log and changes nothing.
    pub fn act(&mut self, action: Action) {
        // The events of a battle's last command are told with the battle as it was: afterwards it is gone.
        let before = self.expedition.battle().cloned();
        let command = Command {
            revision: self.expedition.revision(),
            action,
        };
        match self.expedition.apply(&command) {
            Ok(events) => {
                let battle = self.expedition.battle().or(before.as_ref());
                let lines = events
                    .iter()
                    .filter_map(|e| words::describe_campaign(e, battle));
                self.log.extend(lines);
                self.unsaved = !self.saves.save(&self.expedition);
                self.follow_the_turn();
            }
            Err(refused) => self.log.push(words::refusal_text(&refused)),
        }
        self.waited = 0.0;
    }

    /// An action of the battle's stack whose turn it is.
    pub fn fight(&mut self, action: battle::Action) {
        let Some(battle) = self.expedition.battle() else {
            return;
        };
        let Some(active) = battle.active() else {
            return;
        };
        let order = battle::Command {
            revision: battle.revision(),
            actor: active.stack.clone(),
            action,
        };
        self.act(Action::Battle(order));
    }

    /// The pilot's command for whoever's turn it is in the battle.
    pub fn pilot(&mut self) {
        let order = self
            .expedition
            .battle()
            .and_then(|b| approach_v1(&b.observe()));
        if let Some(order) = order {
            self.act(Action::Battle(order));
        }
    }

    /// Starts the expedition anew, over the save of the old one.
    pub fn restart(&mut self) {
        self.expedition = Expedition::start(Arc::clone(self.expedition.def_shared()));
        self.log = vec![t!("log-new")];
        self.unsaved = !self.saves.save(&self.expedition);
    }

    /// Puts the cursor on the player's stack when its turn in a battle begins.
    fn follow_the_turn(&mut self) {
        let Some(battle) = self.expedition.battle() else {
            return;
        };
        let active = battle.active().and_then(|a| battle.stack(&a.stack));
        if let Some(stack) = active.filter(|s| s.side == Side::Player && !s.out_of_control) {
            self.cursor = stack.cell;
        }
    }

    /// Whether the expedition has ended, one way or another.
    #[must_use]
    pub fn over(&self) -> bool {
        !matches!(self.expedition.status(), Status::Adventure | Status::Battle)
    }

    /// The troops the town sells, in the order of their keys, each with its price and what is left of it.
    #[must_use]
    pub fn on_sale(&self) -> Vec<(&str, u64, u32)> {
        let prices = self.expedition.def().prices.iter();
        prices
            .map(|(troop, price)| (troop.as_str(), *price, self.expedition.stock(troop)))
            .collect()
    }

    /// The band whose contract the key takes: the next standing band after the one whose contract is held, so
    /// that pressing again goes through all of them. `None` when the rules would refuse it.
    #[must_use]
    pub fn contract_offer(&self) -> Option<&str> {
        let expedition = &self.expedition;
        let bands = &expedition.def().encounters;
        let held = expedition.contract();
        let from = bands
            .iter()
            .position(|band| Some(band.object_id.as_str()) == held);
        let order = bands
            .iter()
            .cycle()
            .skip(from.map_or(0, |at| at + 1))
            .take(bands.len());
        let mut standing = order
            .map(|band| band.object_id.as_str())
            .filter(|id| Some(*id) != held);
        standing.find(|id| {
            let take = Action::TakeContract {
                id: (*id).to_owned(),
            };
            expedition.refusal(&take).is_none()
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use bounty_core::campaign::Refused;
    use gamekit::{files, i18n};

    use super::*;
    use crate::content;

    fn scratch(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("bounty-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder.join("expedition.json")
    }

    fn fresh() -> Adventure {
        i18n::init(&words::LANGUAGES);
        Adventure::new(content::expedition(), Saves(None))
    }

    fn step(adventure: &mut Adventure, dx: i32, dy: i32, times: u32) {
        for _ in 0..times {
            adventure.act(Action::Move { dx, dy });
        }
    }

    /// Fights the battle out with the pilot on both sides.
    fn fight_out(adventure: &mut Adventure) {
        while adventure.expedition.status() == Status::Battle {
            adventure.pilot();
        }
    }

    // The whole loop by the commands the screen gives: town, contract, two battles, the clue, the search.
    #[test]
    fn the_expedition_can_be_played_to_its_end() {
        let mut a = fresh();
        assert_eq!(a.on_sale(), [("warden", 20, 10)]);
        assert_eq!(a.contract_offer(), Some("demo-guard"));
        a.act(Action::TakeContract {
            id: "demo-guard".to_owned(),
        });
        assert_eq!(
            a.contract_offer(),
            Some("demo-captain"),
            "the key goes on to the next band"
        );
        a.act(Action::TakeContract {
            id: "demo-captain".to_owned(),
        });
        assert_eq!(a.contract_offer(), Some("demo-guard"));
        a.act(Action::Recruit {
            troop_id: "warden".to_owned(),
            quantity: 1,
        });
        a.act(Action::Recruit {
            troop_id: "warden".to_owned(),
            quantity: 1,
        });
        assert_eq!(a.on_sale(), [("warden", 20, 8)]);

        step(&mut a, 1, 0, 3);
        assert_eq!(a.expedition.status(), Status::Battle);
        let stack = a.expedition.battle().unwrap().stack("p0").unwrap().cell;
        assert_eq!(
            a.cursor, stack,
            "the cursor is on the stack whose turn it is"
        );
        fight_out(&mut a);
        assert_eq!(
            a.contract_offer(),
            None,
            "outside the town no contract is offered"
        );
        step(&mut a, 1, 0, 4);
        fight_out(&mut a);
        assert!(a.expedition.objective_known().is_some());
        step(&mut a, 0, 1, 4);
        a.act(Action::Search);
        assert!(a.over());
        assert_eq!(a.expedition.status(), Status::Victory);
        assert!(!a.unsaved);

        let log = a.log.join("\n");
        for said in [
            "Contract taken",
            "Recruited",
            "A battle begins",
            "Battle won",
            "A clue",
            "The prize is found",
        ] {
            assert!(log.contains(said), "{said} is not in the log:\n{log}");
        }
        assert!(!log.contains("log-") && !log.contains('{'), "{log}");

        // Ended, it takes nothing more and says so.
        let before = a.expedition.clone();
        a.act(Action::WaitTick);
        assert_eq!(a.expedition, before);
        assert_eq!(a.log.last().unwrap(), "The expedition is over");
        a.restart();
        assert!(!a.over());
        assert_eq!(a.expedition.ticks(), 0);
    }

    #[test]
    fn a_refusal_is_told_and_changes_nothing() {
        let mut a = fresh();
        let before = a.expedition.clone();
        a.act(Action::Move { dx: -1, dy: 0 });
        assert_eq!(a.log, ["There is no way there"]);
        a.act(Action::Recruit {
            troop_id: "warden".to_owned(),
            quantity: 6,
        });
        assert!(a.log[1].starts_with("That costs 120"), "{:?}", a.log);
        assert_eq!(a.expedition, before);
        assert_eq!(
            a.expedition.refusal(&Action::Search),
            None,
            "asking whether a command would be refused carries nothing out"
        );
        assert_eq!(
            a.expedition.refusal(&Action::Surrender),
            Some(Refused::NotInBattle)
        );
        assert_eq!(a.expedition, before);
    }

    // Closing the game and starting it again goes on where it stopped, in a battle too.
    #[test]
    fn the_game_goes_on_from_its_save() {
        let file = scratch("continue");
        i18n::init(&words::LANGUAGES);
        let mut a = Adventure::new(content::expedition(), Saves(Some(file.clone())));
        assert!(a.log.is_empty(), "nothing saved yet: a new expedition");
        a.act(Action::Recruit {
            troop_id: "warden".to_owned(),
            quantity: 2,
        });
        step(&mut a, 1, 0, 3);
        assert_eq!(a.expedition.status(), Status::Battle);
        assert!(!a.unsaved);

        let b = Adventure::new(content::expedition(), Saves(Some(file.clone())));
        assert_eq!(b.expedition, a.expedition);
        assert_eq!(b.log, ["The saved expedition goes on"]);
        assert_eq!(b.cursor, a.cursor);

        // The current file torn: the game goes on from the one before it, one command back, and says so.
        let whole = files::read_whole(&file).unwrap();
        std::fs::write(&file, &whole[..whole.len() / 2]).unwrap();
        let c = Adventure::new(content::expedition(), Saves(Some(file.clone())));
        assert_eq!(c.expedition.revision() + 1, a.expedition.revision());
        assert!(c.log[0].starts_with("The last save could not be read"));

        // Both torn: a new expedition, and the torn files are still there for whoever wants to look.
        std::fs::write(files::previous_of(&file), "torn").unwrap();
        let d = Adventure::new(content::expedition(), Saves(Some(file.clone())));
        assert_eq!(d.expedition.revision(), 0);
        assert_eq!(
            files::read_whole(&files::previous_of(&file)).as_deref(),
            Some("torn")
        );
        std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_save_that_cannot_be_written_is_told() {
        let file = scratch("unwritable");
        i18n::init(&words::LANGUAGES);
        // A file where the folder should be: nothing can be written under it.
        std::fs::write(file.parent().unwrap(), "in the way").unwrap();
        let mut a = Adventure::new(content::expedition(), Saves(Some(file.clone())));
        a.act(Action::WaitTick);
        assert!(a.unsaved);
        assert_eq!(a.expedition.ticks(), 1, "the game goes on all the same");
        std::fs::remove_file(file.parent().unwrap()).unwrap();
    }
}
