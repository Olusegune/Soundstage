//! Soundstage core: everything that decides what is allowed, what is rendered, and what is
//! exported. The desktop shell stays thin and calls into this crate.

pub mod adapters;
pub mod assistant;
pub mod audio;
pub mod capability;
pub mod clone;
pub mod consent;
pub mod decode;
pub mod direction;
pub mod director;
pub mod editor;
pub mod engines;
pub mod exporter;
pub mod generate;
pub mod hardware;
pub mod history;
pub mod ids;
pub mod jobrec;
pub mod jobs;
pub mod layout;
pub mod listening;
pub mod licence;
pub mod loudness;
pub mod mix;
pub mod net;
pub mod plan;
pub mod output;
pub mod project;
pub mod projectpack;
pub mod perform;
pub mod pronounce;
pub mod providers;
pub mod render;
pub mod safety;
pub mod secure;
pub mod script;
pub mod tags;
pub mod timeline;
pub mod transcribe;
#[cfg(test)]
pub mod testutil;
pub mod util;
