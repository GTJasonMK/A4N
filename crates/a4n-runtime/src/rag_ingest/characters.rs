mod attributes;
mod foreshadowing;
mod profile_chunks;
mod relationships;
mod states;

pub(crate) use self::attributes::ingest_character_attribute_events_inner;
pub(crate) use self::foreshadowing::ingest_foreshadowing_inner;
pub(crate) use self::profile_chunks::{ingest_character_records_inner, ingest_protagonists_inner};
pub(crate) use self::relationships::ingest_relationships_inner;
pub(crate) use self::states::ingest_character_states_inner;
