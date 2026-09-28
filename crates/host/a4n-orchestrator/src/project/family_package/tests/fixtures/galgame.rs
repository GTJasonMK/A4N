use super::super::*;

pub(in crate::project::family_package::tests) fn derived_galgame_project() -> novel_project::Model {
    let mut project = derived_video_project();
    project.title = "galgame".to_string();
    project.project_kind = PROJECT_KIND_GALGAME_TEST.to_string();
    project
}

pub(in crate::project::family_package::tests) fn package_with_galgame_project()
-> ProjectFamilyPackage {
    let mut package = package_with_derived_project();
    package.projects[1] = derived_galgame_project();
    package.authoring_documents = package.source_snapshot_chapters.iter().map(|source| {
        let document = json!({
            "schema_version": "adaptation_plan_v1", "target": "galgame",
            "intention": "保留现有场景", "decisions": [],
            "units": [{
                "id": "opening", "origin": "historical", "title": "开场",
                "location": null, "participants": [], "dramatic_question": "发生什么",
                "objective": "呈现开场", "obstacle": "未知", "entry_state": "开始", "exit_state": "结束",
                "source_span_ids": [], "beats": [{
                    "id": "narration", "kind": "action", "is_climax": false,
                    "origin": "historical", "importance": "required", "action": "开场旁白",
                    "reaction": null, "dialogue_intent": null, "entry_state": "开始", "exit_state": "结束",
                    "source_span_ids": []
                }]
            }]
        });
        let inputs = json!({
            "sources": [{
                "root_project_id": source.root_project_id,
                "source_chapter_id": source.source_chapter_id,
                "source_snapshot_id": source.snapshot_id,
                "source_snapshot_chapter_id": source.id,
                "content_sha256": source.content_hash
            }],
            "video_reference": null, "dependencies": []
        });
        a4n_db_models::models::project::media_authoring_document::Model {
            id: 1400 + source.id,
            project_id: 2,
            source_snapshot_id: source.snapshot_id,
            source_chapter_id: Some(source.source_chapter_id),
            kind: "adaptation_plan".into(), version: 1, revision: 1, status: "adopted".into(),
            document_sha256: a4n_db::generation::canonical_json_sha256(&(&inputs, &document)).unwrap(),
            inputs_json: inputs,
            document_json: document,
            created_at: dt(), updated_at: dt(),
        }
    }).collect();
    package.galgame_scripts = vec![galgame_script_record()];
    package.galgame_scenes = vec![galgame_scene_record(1001)];
    package.galgame_lines = vec![galgame_line_record(2201)];
    package
}

pub(in crate::project::family_package::tests) fn galgame_script_record() -> galgame_script::Model {
    galgame_script::Model {
        id: 2101,
        project_id: 2,
        root_project_id: 1,
        source_snapshot_id: 901,
        branch_plan_id: None,
        title: "Galgame 剧本".to_string(),
        style_config_json: json!({}),
        status: "draft".to_string(),
        created_at: dt(),
        updated_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn galgame_scene_record(
    source_snapshot_chapter_id: i64,
) -> galgame_scene::Model {
    galgame_scene_record_with_id(2201, source_snapshot_chapter_id, "main", None)
}

pub(in crate::project::family_package::tests) fn galgame_scene_record_with_id(
    id: i64,
    source_snapshot_chapter_id: i64,
    entry_kind: &str,
    next_scene_id: Option<i64>,
) -> galgame_scene::Model {
    galgame_scene::Model {
        id,
        script_id: 2101,
        source_snapshot_chapter_id,
        adaptation_plan_id: Some(1400 + source_snapshot_chapter_id),
        adaptation_unit_id: Some("opening".into()),
        scene_index: 1,
        title: "开场".to_string(),
        location: None,
        time_of_day: None,
        participants_json: json!([]),
        background_note: None,
        entry_kind: entry_kind.to_string(),
        next_scene_id,
        created_at: dt(),
        updated_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn galgame_line_record(
    scene_id: i64,
) -> galgame_line::Model {
    galgame_line_record_with_id(2301, scene_id)
}

pub(in crate::project::family_package::tests) fn galgame_line_record_with_id(
    id: i64,
    scene_id: i64,
) -> galgame_line::Model {
    galgame_line::Model {
        id,
        scene_id,
        line_index: 1,
        line_type: "narration".to_string(),
        speaker_profile_id: None,
        text: "这里是开场旁白。".to_string(),
        expression: None,
        pose: None,
        position: None,
        presentation_json: empty_galgame_presentation_json(),
        text_anchor_id: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn empty_galgame_presentation_json() -> Value {
    json!({
        "schema_version": "galgame_line_presentation_v3", "origin": "historical", "beat_ref": null,
        "visible_cast": [],
        "enter": [],
        "exit": [],
        "focus_character_profile_id": null,
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
    })
}

pub(in crate::project::family_package::tests) fn speaker_galgame_presentation_json(
    profile_id: i64,
    expression: &str,
    position: &str,
) -> Value {
    let mut presentation = empty_galgame_presentation_json();
    presentation["visible_cast"] = json!([{
        "character_profile_id": profile_id,
        "expression": expression,
        "pose": "standing",
        "position": position,
        "facing": "front",
        "layer": "front"
    }]);
    presentation["focus_character_profile_id"] = json!(profile_id);
    presentation
}

pub(in crate::project::family_package::tests) fn set_galgame_speaker_line(
    line: &mut galgame_line::Model,
    profile_id: i64,
    expression: &str,
    position: &str,
) {
    line.line_type = "dialogue".to_string();
    line.speaker_profile_id = Some(profile_id);
    line.expression = Some(expression.to_string());
    line.pose = Some("standing".to_string());
    line.position = Some(position.to_string());
    line.presentation_json = speaker_galgame_presentation_json(profile_id, expression, position);
}

pub(in crate::project::family_package::tests) fn galgame_choice_record(
    id: i64,
    line_id: i64,
    target_scene_id: i64,
) -> galgame_choice::Model {
    galgame_choice_record_with_index(id, line_id, 1, target_scene_id)
}

pub(in crate::project::family_package::tests) fn galgame_choice_record_with_index(
    id: i64,
    line_id: i64,
    option_index: i32,
    target_scene_id: i64,
) -> galgame_choice::Model {
    galgame_choice::Model {
        id,
        line_id,
        option_index,
        option_text: "前往".to_string(),
        target_scene_id: Some(target_scene_id),
        created_at: dt(),
    }
}
