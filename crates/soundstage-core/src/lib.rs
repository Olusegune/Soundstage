//! Soundstage core: everything that decides what is allowed, what is rendered, and what is
//! exported. The desktop shell stays thin and calls into this crate.

pub mod adapters;
pub mod audio;
pub mod consent;
pub mod direction;
pub mod exporter;
pub mod layout;
pub mod licence;
pub mod loudness;
pub mod mix;
pub mod project;
pub mod providers;
pub mod render;
pub mod safety;
pub mod script;
pub mod tags;
pub mod util;
