use a4n_core::models::{
    character::protagonist_profile,
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    media::media_asset,
    project::{source_snapshot_chapter, text_anchor},
};
use a4n_db::galgame::GalgameGenerationSpecUpsertInput;
use chrono::Utc;
use serde_json::{Value, json};

use super::super::{
    GALGAME_ENTITY_SCENE, GALGAME_TARGET_BACKGROUND, LINE_TYPE_DIALOGUE, SCENE_ENTRY_MAIN,
    SCRIPT_STATUS_READY,
};

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

pub(super) fn test_snapshot_chapter(
    id: i64,
    chapter_number: i32,
    content: &str,
) -> source_snapshot_chapter::Model {
    source_snapshot_chapter::Model {
        id,
        snapshot_id: 20,
        root_project_id: 1,
        source_chapter_id: 2001,
        chapter_number,
        title: Some("测试章节".to_string()),
        content: content.to_string(),
        summary: Some("章节摘要".to_string()),
        analysis_data_json: None,
        content_hash: "hash".to_string(),
        analysis_hash: None,
    }
}

pub(super) fn test_text_anchor(
    id: i64,
    source_snapshot_chapter_id: i64,
    paragraph_index: i32,
    start_offset: i32,
    end_offset: i32,
    quote: &str,
) -> text_anchor::Model {
    text_anchor::Model {
        id,
        project_id: 1,
        source_chapter_id: Some(2001),
        source_snapshot_id: Some(20),
        source_snapshot_chapter_id: Some(source_snapshot_chapter_id),
        source_kind: "chapter".to_string(),
        source_id: 2001,
        paragraph_index,
        start_offset,
        end_offset,
        quote: quote.to_string(),
        quote_hash: "quote_hash".to_string(),
        source_version_hash: "source_hash".to_string(),
        created_at: Utc::now().naive_utc(),
    }
}

pub(super) fn test_profile(id: i64, name: &str) -> protagonist_profile::Model {
    let now = Utc::now().naive_utc();
    protagonist_profile::Model {
        id,
        project_id: 1,
        name: name.to_string(),
        gender: None,
        age: None,
        base_attributes: json!({}),
        current_state: json!({}),
        character_arc: None,
        voice_fingerprint: None,
        appearance: None,
        background: None,
        personality: None,
        last_synced_chapter: 0,
        created_at: now,
        updated_at: now,
    }
}

pub(super) fn test_script(id: i64) -> galgame_script::Model {
    let now = Utc::now().naive_utc();
    galgame_script::Model {
        id,
        project_id: 10,
        root_project_id: 1,
        source_snapshot_id: 20,
        title: "测试 Galgame".to_string(),
        style_config_json: json!({}),
        status: SCRIPT_STATUS_READY.to_string(),
        created_at: now,
        updated_at: now,
    }
}

pub(super) fn test_scene(id: i64, scene_index: i32) -> galgame_scene::Model {
    let now = Utc::now().naive_utc();
    galgame_scene::Model {
        id,
        script_id: 1,
        source_snapshot_chapter_id: 20,
        scene_index,
        title: "雨夜车站".to_string(),
        location: Some("车站".to_string()),
        time_of_day: Some("夜晚".to_string()),
        participants_json: json!([]),
        background_note: None,
        entry_kind: SCENE_ENTRY_MAIN.to_string(),
        next_scene_id: None,
        created_at: now,
        updated_at: now,
    }
}

pub(super) fn test_choice(
    id: i64,
    line_id: i64,
    option_index: i32,
    option_text: &str,
) -> galgame_choice::Model {
    galgame_choice::Model {
        id,
        line_id,
        option_index,
        option_text: option_text.to_string(),
        target_scene_id: Some(30),
        created_at: Utc::now().naive_utc(),
    }
}

pub(super) fn test_media_asset(id: i64) -> media_asset::Model {
    let now = Utc::now().naive_utc();
    media_asset::Model {
        id,
        project_id: 10,
        root_project_id: 1,
        source_snapshot_id: Some(20),
        source_chapter_id: Some(2001),
        source_snapshot_chapter_id: Some(20),
        media_kind: "image".to_string(),
        origin: "generated".to_string(),
        file_path: Some("galgame/background.png".to_string()),
        thumbnail_path: None,
        width: Some(1280),
        height: Some(720),
        duration_secs: None,
        fps: None,
        file_size: Some(1024),
        checksum: Some("checksum".to_string()),
        status: "completed".to_string(),
        error_message: None,
        created_at: now,
        updated_at: now,
    }
}
