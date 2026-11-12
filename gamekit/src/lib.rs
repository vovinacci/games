//! What the games here share around their rules: the virtual screen, text, translations, the game's own files, the
//! dedication, and aids for developers. A game's rules never depend on this crate; only its application does.
//!
//! `i18n`, `files` and the command line part of `dev` do not touch the window, so they work in tests and tools.

pub mod dedication;
pub mod dev;
pub mod files;
pub mod i18n;
pub mod screen;
pub mod text;
