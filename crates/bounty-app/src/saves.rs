//! The saved expedition in the user's data folder: one file, and the one before it.

use std::path::PathBuf;
use std::sync::Arc;

use bounty_core::campaign::{Expedition, ExpeditionDef};
use bounty_core::save;
use gamekit::files;

/// Where the expedition is saved; `None` in a developer run that must not touch the user's files.
pub struct Saves(pub Option<PathBuf>);

impl Saves {
    /// The user's own file for this expedition under these rules: each has its own, so that playing one never
    /// touches the save of another.
    #[must_use]
    pub fn of_the_user(def: &ExpeditionDef) -> Self {
        let name = format!("save-{}-{}.json", def.id, def.rules.id);
        Self(files::data_file("bounty", &name))
    }

    /// The saved expedition of `def`, with whether it comes from the file before the current one. `None` when
    /// there is no save this game can go on from; the files stay as they are.
    #[must_use]
    pub fn load(&self, def: &Arc<ExpeditionDef>) -> Option<(Expedition, bool)> {
        let read = |text: &str| save::decode(text, Arc::clone(def)).ok();
        files::read_newest(self.0.as_deref()?, read)
    }

    /// Saves the expedition. `false` when it could not be saved; a run that keeps no files has nothing to fail.
    pub fn save(&self, expedition: &Expedition) -> bool {
        let Some(file) = &self.0 else {
            return true;
        };
        let def = expedition.def_shared();
        let valid = |text: &str| save::decode(text, Arc::clone(def)).is_ok();
        match files::write_keeping_previous(file, &save::encode(expedition), valid) {
            Ok(()) => true,
            Err(error) => {
                eprintln!("expedition not saved to {}: {error}", file.display());
                false
            }
        }
    }
}
