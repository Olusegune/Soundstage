//! Soundstage core: everything that decides what is allowed, what is rendered, and what is
//! exported. The desktop shell stays thin and calls into this crate.

pub mod adapters;
pub mod audio;
pub mod consent;
pub mod decode;
pub mod direction;
pub mod engines;
pub mod exporter;
pub mod generate;
pub mod jobs;
pub mod layout;
pub mod licence;
pub mod loudness;
pub mod mix;
pub mod net;
pub mod output;
pub mod project;
pub mod providers;
pub mod render;
pub mod safety;
pub mod script;
pub mod tags;
#[cfg(test)]
pub mod testutil;
pub mod util;
