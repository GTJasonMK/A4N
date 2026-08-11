use std::collections::BTreeSet;

use a4n_core::error::AppError;
use a4n_domain::shooting_script::ShootingSceneV1;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    analysis::diagnose_structured_json_with_schema, shape::json_shape_schema_from_example,
};
use crate::structured_llm::schema_repair::contract::{
    SchemaFieldPatchOperationV1, StructuredSchemaReport,
};

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Fixture {
    names: Vec<String>,
    mode: Mode,
    nested: Nested,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Mode {
    First,
    Second,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Nested {
    title: String,
}

fn diagnose_structured_json<T>(value: &Value) -> Result<StructuredSchemaReport, AppError>
where
    T: JsonSchema,
{
    let schema = serde_json::to_value(schemars::schema_for!(T))?;
    diagnose_structured_json_with_schema(value, schema)
}

#[test]
fn collects_multiple_repairable_schema_errors() {
    let report = diagnose_structured_json::<Fixture>(&json!({
        "names": "Alice",
        "mode": "third",
        "nested": "wrong",
        "extra": true
    }))
    .expect("schema diagnostics");

    assert!(report.field_patch_allowed);
    assert_eq!(report.diagnostics.len(), 4);
    assert!(report.diagnostics.iter().any(|item| item.path == "/names"));
    assert!(report.diagnostics.iter().any(|item| item.path == "/mode"));
    assert!(report.diagnostics.iter().any(|item| item.path == "/nested"));
    assert!(report.diagnostics.iter().any(|item| item.path == "/extra"));
}

#[test]
fn reports_missing_required_field_as_add() {
    let report = diagnose_structured_json::<Fixture>(&json!({
        "names": [],
        "mode": "first"
    }))
    .expect("schema diagnostics");

    let diagnostic = report
        .diagnostics
        .iter()
        .find(|item| item.path == "/nested")
        .expect("missing nested diagnostic");
    assert_eq!(diagnostic.operation, Some(SchemaFieldPatchOperationV1::Add));
    assert!(diagnostic.expected_schema.is_object());
}

#[test]
fn derives_strict_schema_from_prompt_shape_example() {
    let schema = json_shape_schema_from_example(&json!({
        "mode": "first|second",
        "items": [{"name": "string"}],
        "count": 1
    }));

    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["mode"]["enum"],
        json!(["first", "second"])
    );
    assert_eq!(schema["properties"]["items"]["items"]["type"], "object");
    assert_eq!(schema["properties"]["count"]["type"], "integer");
}

#[test]
fn shooting_scene_collects_all_string_instead_of_array_errors() {
    let value = json!({
        "scene_index": 1,
        "scene_identity": "classroom",
        "sequence_index": 1,
        "narrative_unit_index": 1,
        "timeline_position": 1,
        "intercut_group_id": null,
        "editing_mode": "continuity",
        "transition_contracts": [],
        "title": "课堂",
        "slugline": "内景 教室 日",
        "location": "教室",
        "time_of_day": "日",
        "participants": ["林清辞"],
        "focal_character": "林清辞",
        "source_span_ids": ["span-1"],
        "dramatic_question": "她是否行动",
        "scene_objective": "维持距离",
        "scene_obstacle": "不安",
        "relationship_before": "疏远",
        "relationship_after": "仍然疏远",
        "audience_knowledge_before": "她暗恋两年",
        "audience_knowledge_after": "她准备写纸条",
        "withheld_information": "纸条是否送出",
        "viewpoint": {
            "mode": "limited_third_person",
            "anchor_character": "林清辞",
            "information_strategy": "限制信息"
        },
        "spatial_continuity": {
            "continuity_intent": "transparent",
            "primary_axis": null,
            "screen_direction": null,
            "reorientation_strategy": null
        },
        "sound_strategy": "教室环境声",
        "entry_state": "观察",
        "exit_state": "犹豫",
        "shots": []
    });

    let report =
        diagnose_structured_json::<ShootingSceneV1>(&value).expect("shooting scene diagnostics");
    let paths = report
        .diagnostics
        .iter()
        .map(|item| item.path.as_str())
        .collect::<BTreeSet<_>>();
    assert!(report.field_patch_allowed);
    assert!(paths.contains("/audience_knowledge_before"));
    assert!(paths.contains("/audience_knowledge_after"));
    assert!(paths.contains("/withheld_information"));
}
