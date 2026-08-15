use super::super::*;

pub(in crate::project::family_package::tests) fn derived_galgame_project() -> novel_project::Model {
    let mut project = derived_video_project();
    project.title = "galgame".to_string();
    project.project_kind = PROJECT_KIND_GALGAME_TEST.to_string();
    project
}

pub(in crate::project::family_package::tests) fn package_with_galgame_project()
-> ProjectFamilyPackage {
    let mut package = package_with_part_outlines(Vec::new());
    let snapshot_chapters = (1..=10)
        .map(source_snapshot_chapter_record)
        .collect::<Vec<_>>();
    let mut snapshot = source_snapshot_record();
    snapshot.chapter_hashes_json = Value::Array(
        snapshot_chapters
            .iter()
            .map(source_snapshot_chapter_hash_row)
            .collect(),
    );
    package.projects.push(derived_galgame_project());
    package.source_snapshots = vec![snapshot];
    package.source_snapshot_chapters = snapshot_chapters;
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
        "schema_version": "galgame_line_presentation_v2",
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
) -> Value {
    let mut presentation = empty_galgame_presentation_json();
    presentation["visible_cast"] = json!([{
        "character_profile_id": profile_id,
        "expression": expression,
        "pose": "standing",
        "position": "center",
        "facing": "front",
        "layer": "front"
    }]);
    presentation["focus_character_profile_id"] = json!(profile_id);
    presentation
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
