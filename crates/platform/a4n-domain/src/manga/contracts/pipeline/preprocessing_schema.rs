use schemars::{Schema, SchemaGenerator};
use serde_json::{Value, json};

fn text() -> Value {
    json!({"type":"string","minLength":1,"pattern":"\\S"})
}
fn strings() -> Value {
    json!({"type":"array","minItems":1,"items":text()})
}
fn schema(value: Value) -> Schema {
    value.try_into().expect("static preprocessing schema")
}

pub(super) fn source_events(_: &mut SchemaGenerator) -> Schema {
    schema(
        json!({"type":"object","required":["events"],"properties":{"events":{"type":"array","minItems":1,"items":{
            "type":"object","required":["event_id","type","description","participants","source_span_ids"],
            "properties":{"event_id":text(),"type":text(),"description":text(),"participants":{"type":"array","items":text()},"source_span_ids":strings()}
        }}}}),
    )
}
pub(super) fn characters(_: &mut SchemaGenerator) -> Schema {
    schema(json!({"type":"object"}))
}
pub(super) fn speech(_: &mut SchemaGenerator) -> Schema {
    let entries = json!({"type":"array","items":{"type":"object","required":["source_event_id"],"properties":{"source_event_id":text()}}});
    schema(
        json!({"type":"object","required":["dialogues","narrations"],"properties":{"dialogues":entries,"narrations":entries}}),
    )
}
pub(super) fn visual_anchors(_: &mut SchemaGenerator) -> Schema {
    schema(
        json!({"type":"object","required":["scenes"],"properties":{"scenes":{"type":"array","items":{"type":"object","required":["source_event_ids"],"properties":{"source_event_ids":strings()}}}}}),
    )
}
pub(super) fn items(_: &mut SchemaGenerator) -> Schema {
    schema(
        json!({"type":"object","required":["items"],"properties":{"items":{"type":"array","items":{"type":"object","required":["first_appearance_source_event_id"],"properties":{"first_appearance_source_event_id":text()}}}}}),
    )
}
pub(super) fn mood_progression(_: &mut SchemaGenerator) -> Schema {
    schema(json!({"type":"array","minItems":3,"maxItems":6,"items":text()}))
}
