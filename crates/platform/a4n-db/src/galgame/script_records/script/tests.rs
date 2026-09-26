use super::persist_galgame_script_record;
use crate::galgame::{GalgameScriptRecordError, PersistGalgameScriptRecordInput};
use a4n_db_models::models::{
    chapter::chapter,
    galgame::{galgame_choice, galgame_line, galgame_scene},
    project::source_snapshot_chapter,
};
use a4n_domain::project::ProjectKind;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;

#[tokio::test]
async fn persist_script_rejects_empty_scenes_before_database_access() {
    let input = PersistGalgameScriptRecordInput {
        project_id: 1,
        root_project_id: 1,
        source_snapshot_id: 2,
        title: "测试 Galgame".to_string(),
        style_config_json: json!({}),
        status: "draft".to_string(),
        scenes: Vec::new(),
    };

    assert!(matches!(
        persist_galgame_script_record(&DatabaseConnection::Disconnected, input).await,
        Err(GalgameScriptRecordError::InvalidArgument(message))
            if message == "Galgame persist_script 至少需要一个场"
    ));
}

#[tokio::test]
async fn persist_script_writes_scene_line_choice_graph_in_one_contract() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "Galgame 批量写入根项目".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let _chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let derived = crate::project::create_derived_project_record(
        &db,
        1,
        crate::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: ProjectKind::Galgame.as_str().to_string(),
            title: "Galgame 批量写入".to_string(),
            description: String::new(),
            chapter_scope: crate::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    let snapshot_chapter = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(derived.source_snapshot_id.unwrap()))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let presentation = json!({
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
    });
    let script = crate::galgame::replace_galgame_script_for_project_record(
        &db,
        derived.id,
        PersistGalgameScriptRecordInput {
            project_id: derived.id,
            root_project_id: root.id,
            source_snapshot_id: derived.source_snapshot_id.unwrap(),
            title: "批量合同脚本".to_string(),
            style_config_json: json!({}),
            status: "draft".to_string(),
            scenes: vec![
                crate::galgame::PersistGalgameSceneRecordInput {
                    scene_index: 1,
                    source_snapshot_chapter_id: snapshot_chapter.id,
                    shooting_scene_identity: Some("unit:shot-1".to_string()),
                    title: "序幕".to_string(),
                    location: Some("大厅".to_string()),
                    time_of_day: None,
                    participants_json: json!([]),
                    background_note: None,
                    entry_kind: "main".to_string(),
                    next_scene_index: Some(2),
                    lines: vec![
                        crate::galgame::PersistGalgameLineRecordInput {
                            line_index: 1,
                            line_type: "narration".to_string(),
                            speaker_profile_id: None,
                            text: "灯亮了。".to_string(),
                            expression: None,
                            pose: None,
                            position: None,
                            presentation_json: presentation.clone(),
                            text_anchor_id: None,
                            choices: Vec::new(),
                        },
                        crate::galgame::PersistGalgameLineRecordInput {
                            line_index: 2,
                            line_type: "choice".to_string(),
                            speaker_profile_id: None,
                            text: "接下来呢？".to_string(),
                            expression: None,
                            pose: None,
                            position: None,
                            presentation_json: presentation.clone(),
                            text_anchor_id: None,
                            choices: vec![
                                crate::galgame::PersistGalgameChoiceRecordInput {
                                    option_index: 1,
                                    option_text: "前进".to_string(),
                                    target_scene_index: 2,
                                },
                                crate::galgame::PersistGalgameChoiceRecordInput {
                                    option_index: 2,
                                    option_text: "观察".to_string(),
                                    target_scene_index: 2,
                                },
                            ],
                        },
                    ],
                },
                crate::galgame::PersistGalgameSceneRecordInput {
                    scene_index: 2,
                    shooting_scene_identity: None,
                    source_snapshot_chapter_id: snapshot_chapter.id,
                    title: "回收".to_string(),
                    location: Some("走廊".to_string()),
                    time_of_day: None,
                    participants_json: json!([]),
                    background_note: None,
                    entry_kind: "main".to_string(),
                    next_scene_index: None,
                    lines: vec![crate::galgame::PersistGalgameLineRecordInput {
                        line_index: 1,
                        line_type: "narration".to_string(),
                        speaker_profile_id: None,
                        text: "故事继续。".to_string(),
                        expression: None,
                        pose: None,
                        position: None,
                        presentation_json: presentation,
                        text_anchor_id: None,
                        choices: Vec::new(),
                    }],
                },
            ],
        },
    )
    .await
    .unwrap();

    let scenes = galgame_scene::Entity::find()
        .filter(galgame_scene::Column::ScriptId.eq(script.id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(scenes.len(), 2);
    assert_eq!(scenes[0].next_scene_id, Some(scenes[1].id));
    let lines = galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.is_in(scenes.iter().map(|scene| scene.id)))
        .order_by_asc(galgame_line::Column::Id)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(lines.len(), 3);
    let reordered = crate::galgame::reorder_galgame_line_records_transaction(
        &db,
        scenes[0].id,
        vec![lines[1].id, lines[0].id],
    )
    .await
    .unwrap();
    assert_eq!(
        reordered
            .iter()
            .map(|line| (line.id, line.line_index))
            .collect::<Vec<_>>(),
        vec![(lines[1].id, 1), (lines[0].id, 2)]
    );
    let choices = galgame_choice::Entity::find()
        .filter(galgame_choice::Column::LineId.eq(lines[1].id))
        .order_by_asc(galgame_choice::Column::OptionIndex)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].target_scene_id, Some(scenes[1].id));
    assert_eq!(choices[1].target_scene_id, Some(scenes[1].id));

    crate::galgame::delete_galgame_line_with_generation_media_graph_record(
        &db,
        derived.id,
        lines[0].id,
    )
    .await
    .unwrap();
    let remaining_lines = galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.eq(scenes[0].id))
        .order_by_asc(galgame_line::Column::LineIndex)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(
        remaining_lines
            .iter()
            .map(|line| (line.id, line.line_index))
            .collect::<Vec<_>>(),
        vec![(lines[1].id, 1)]
    );
}
