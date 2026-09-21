use a4n_db::galgame::GalgameGenerationSpecUpsertInput;
use a4n_db::galgame::GalgamePipelinePromptTextAnchorRecord;
use a4n_db_models::models::{galgame::galgame_line, generation::generation_spec};
use chrono::Utc;
use serde_json::{Value, json};

use super::super::{GALGAME_ENTITY_SCENE, GALGAME_TARGET_BACKGROUND, LINE_TYPE_DIALOGUE};

pub(super) fn test_line_with(
    id: i64,
    text_anchor_id: Option<i64>,
    expression: Option<&str>,
    pose: Option<&str>,
    position: Option<&str>,
) -> galgame_line::Model {
    let now = Utc::now().naive_utc();
    galgame_line::Model {
        id,
        scene_id: 1,
        line_index: 1,
        line_type: LINE_TYPE_DIALOGUE.to_string(),
        speaker_profile_id: Some(1),
        text: "测试台词".to_string(),
        expression: expression.map(str::to_string),
        pose: pose.map(str::to_string),
        position: position.map(str::to_string),
        presentation_json: json!({
            "schema_version": "galgame_line_presentation_v2",
            "visible_cast": [{
                "character_profile_id": 1,
                "expression": expression,
                "pose": pose,
                "position": position,
                "facing": "front",
                "layer": "front"
            }],
            "enter": [],
            "exit": [],
            "focus_character_profile_id": 1,
            "hold": false,
            "player_knowledge": [],
            "cg": null,
            "cg_note": null,
            "cg_importance": null,
            "transition": null,
            "branch": null,
            "branch_id": null,
            "returns_to_scene_index": null,
            "after_beat_ref": null
        }),
        text_anchor_id,
        created_at: now,
        updated_at: now,
    }
}

pub(super) fn test_spec_input(params_json: Value) -> GalgameGenerationSpecUpsertInput {
    GalgameGenerationSpecUpsertInput {
        canonical_key: "galgame:10:background:scene:100".to_string(),
        project_id: 10,
        root_project_id: 1,
        source_snapshot_id: 20,
        source_chapter_id: Some(2001),
        source_snapshot_chapter_id: Some(1001),
        target_kind: GALGAME_TARGET_BACKGROUND.to_string(),
        target_entity_kind: GALGAME_ENTITY_SCENE.to_string(),
        target_entity_id: Some(100),
        params_json,
    }
}

pub(super) fn test_generation_spec(
    input: &GalgameGenerationSpecUpsertInput,
    source_version_hash: &str,
) -> generation_spec::Model {
    let now = Utc::now().naive_utc();
    generation_spec::Model {
        id: 301,
        project_id: input.project_id,
        root_project_id: input.root_project_id,
        source_context_kind: Some("snapshot_backed".to_string()),
        source_snapshot_id: Some(input.source_snapshot_id),
        source_chapter_id: input.source_chapter_id,
        source_snapshot_chapter_id: input.source_snapshot_chapter_id,
        canvas_text_asset_id: None,
        target_kind: input.target_kind.clone(),
        target_entity_kind: input.target_entity_kind.clone(),
        target_entity_id: input.target_entity_id,
        status: "draft".to_string(),
        source_version_hash: source_version_hash.to_string(),
        params_json: input.params_json.clone(),
        created_at: now,
        updated_at: now,
    }
}

pub(super) fn test_text_anchor(
    id: i64,
    source_snapshot_chapter_id: i64,
    paragraph_index: i32,
    start_offset: i32,
    end_offset: i32,
    quote: &str,
) -> GalgamePipelinePromptTextAnchorRecord {
    GalgamePipelinePromptTextAnchorRecord {
        id,
        source_snapshot_chapter_id: Some(source_snapshot_chapter_id),
        paragraph_index,
        start_offset,
        end_offset,
        quote: quote.to_string(),
    }
}
