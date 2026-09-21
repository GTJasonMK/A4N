mod foreshadowing;
mod key_events;
mod metadata;

pub(super) use foreshadowing::materialize_foreshadowing_source_spans;
pub(super) use key_events::materialize_key_event_source_spans;
pub(super) use metadata::materialize_metadata_source_spans;
