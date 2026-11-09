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

fn object(properties: Value) -> Value {
    let required = properties
        .as_object()
        .expect("static properties")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

fn entries(item: Value) -> Value {
    json!({"type":"array","items":item})
}
fn index() -> Value {
    json!({"type":"integer","minimum":0})
}
fn nullable_text() -> Value {
    json!({"type":["string","null"]})
}

pub(super) fn extracted_characters(_: &mut SchemaGenerator) -> Schema {
    schema(json!({"type":"object","additionalProperties":object(json!({
        "name":text(),"appearance":text(),"personality":text(),
        "role":{"enum":["protagonist","antagonist","supporting","minor","background"]},
        "gender":{"enum":["male","female","unknown"]},"age_description":text(),"source_span_ids":strings()
    }))}))
}

pub(super) fn extracted_speech(_: &mut SchemaGenerator) -> Schema {
    let mut dialogues = entries(object(json!({
        "index":index(),"speaker":text(),"content":text(),"source_event_id":text(),"emotion":text(),
        "target":nullable_text(),"is_internal":{"type":"boolean"},"bubble_type":{"enum":["normal","thought"]}
    })));
    dialogues["maxItems"] = json!(24);
    dialogues["items"]["properties"]["content"]["maxLength"] = json!(30);
    let mut narrations = entries(object(json!({
        "index":index(),"content":text(),"source_event_id":text(),"narration_type":text(),"position":text()
    })));
    narrations["maxItems"] = json!(8);
    narrations["items"]["properties"]["content"]["maxLength"] = json!(40);
    schema(object(
        json!({"dialogues":dialogues,"narrations":narrations}),
    ))
}

pub(super) fn extracted_visual_anchors(_: &mut SchemaGenerator) -> Schema {
    schema(object(json!({"scenes":entries(object(json!({
        "index":index(),"location":text(),"location_en":text(),"time_of_day":text(),
        "atmosphere":text(),"weather":nullable_text(),"lighting":text(),
        "indoor_outdoor":{"enum":["indoor","outdoor"]},"description":text(),
        "source_event_ids":strings(),"source_span_ids":strings()
    })))})))
}

pub(super) fn extracted_items(_: &mut SchemaGenerator) -> Schema {
    schema(object(json!({"items":entries(object(json!({
        "name":text(),"description":text(),"description_en":text(),"importance":text(),
        "first_appearance_source_event_id":text(),"visual_features":text(),"source_span_ids":strings()
    })))})))
}
