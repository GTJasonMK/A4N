//! Explicit migration adapters. Normal generation never reconstructs a film
//! script, invents a missing source, or accepts retired beat-reference formats.

pub mod film;
pub mod galgame;
pub mod manga;
pub mod shooting_script;
pub mod source_sync;
pub mod workflow;

pub use galgame::{
    HistoricalGalgameLine, HistoricalGalgameScene, historical_galgame_beat_id,
    historical_galgame_unit, historical_galgame_unit_id, upgrade_galgame_presentation,
    upgrade_galgame_spec_presentations,
};
pub use manga::{
    LegacyMangaSource, historical_manga_page, saved_manga_source, upgrade_manga_layout,
    upgrade_saved_manga_layout,
};
pub use shooting_script::{
    LegacyShootingBeatRef, LegacyShootingIdentity, adaptation_from_shooting_script,
};
