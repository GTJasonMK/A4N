use crate::galgame::{
    GALGAME_SCENE_ENTRY_BRANCH, GALGAME_SCENE_ENTRY_MAIN, GalgameScriptRecordError,
    PersistGalgameScriptRecordInput,
};
use a4n_core::models::galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script};
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use std::collections::BTreeMap;

pub(super) async fn persist_galgame_script_record<C>(
    db: &C,
    input: PersistGalgameScriptRecordInput,
) -> Result<galgame_script::Model, GalgameScriptRecordError>
where
    C: ConnectionTrait,
{
    if input.scenes.is_empty() {
        return Err(GalgameScriptRecordError::InvalidArgument(
            "Galgame persist_script 至少需要一个场".to_string(),
        ));
    }

    let now = Utc::now().naive_utc();
    let script = galgame_script::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_snapshot_id: Set(input.source_snapshot_id),
        title: Set(input.title),
        style_config_json: Set(input.style_config_json),
        status: Set(input.status),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(GalgameScriptRecordError::Database)?;

    let mut scene_id_by_index = BTreeMap::<i32, i64>::new();
    let mut scene_entry_kind_by_index = BTreeMap::<i32, String>::new();
    let mut pending_scene_next = Vec::new();
    let mut pending_choices = Vec::new();
    for scene_input in input.scenes {
        let scene_index = scene_input.scene_index;
        let next_scene_index = scene_input.next_scene_index;
        let entry_kind = scene_input.entry_kind;
        let scene = galgame_scene::ActiveModel {
            script_id: Set(script.id),
            source_snapshot_chapter_id: Set(scene_input.source_snapshot_chapter_id),
            scene_index: Set(scene_index),
            title: Set(scene_input.title),
            location: Set(scene_input.location),
            time_of_day: Set(scene_input.time_of_day),
            participants_json: Set(scene_input.participants_json),
            background_note: Set(scene_input.background_note),
            entry_kind: Set(entry_kind.clone()),
            next_scene_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(GalgameScriptRecordError::Database)?;

        scene_id_by_index.insert(scene_index, scene.id);
        scene_entry_kind_by_index.insert(scene_index, entry_kind);
        if let Some(next_scene_index) = next_scene_index {
            pending_scene_next.push((scene.id, scene_index, next_scene_index));
        }

        for line_input in scene_input.lines {
            let choices = line_input.choices;
            let line = galgame_line::ActiveModel {
                scene_id: Set(scene.id),
                line_index: Set(line_input.line_index),
                line_type: Set(line_input.line_type),
                speaker_profile_id: Set(line_input.speaker_profile_id),
                text: Set(line_input.text),
                expression: Set(line_input.expression),
                pose: Set(line_input.pose),
                position: Set(line_input.position),
                presentation_json: Set(line_input.presentation_json),
                text_anchor_id: Set(line_input.text_anchor_id),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(GalgameScriptRecordError::Database)?;
            if !choices.is_empty() {
                pending_choices.push((line.id, choices));
            }
        }
    }

    for (scene_id, scene_index, next_scene_index) in pending_scene_next {
        let next_scene_id = scene_id_by_index
            .get(&next_scene_index)
            .copied()
            .ok_or_else(|| {
                GalgameScriptRecordError::InvalidArgument(format!(
                    "Galgame scene next_scene_index 不存在: scene_index={scene_index}, next_scene_index={next_scene_index}"
                ))
            })?;
        if scene_entry_kind_by_index
            .get(&scene_index)
            .map(String::as_str)
            == Some(GALGAME_SCENE_ENTRY_BRANCH)
            && scene_entry_kind_by_index
                .get(&next_scene_index)
                .map(String::as_str)
                != Some(GALGAME_SCENE_ENTRY_MAIN)
        {
            return Err(GalgameScriptRecordError::InvalidArgument(format!(
                "Galgame 支线场 next_scene_index 必须指向主线场: scene_index={scene_index}, next_scene_index={next_scene_index}"
            )));
        }
        galgame_scene::Entity::update_many()
            .col_expr(
                galgame_scene::Column::NextSceneId,
                Expr::value(next_scene_id),
            )
            .filter(galgame_scene::Column::Id.eq(scene_id))
            .exec(db)
            .await
            .map_err(GalgameScriptRecordError::Database)?;
    }

    for (line_id, choices) in pending_choices {
        for choice in choices {
            let target_scene_id = scene_id_by_index
                .get(&choice.target_scene_index)
                .copied()
                .ok_or_else(|| {
                    GalgameScriptRecordError::InvalidArgument(format!(
                        "Galgame choice target_scene_index 不存在: {}",
                        choice.target_scene_index
                    ))
                })?;
            galgame_choice::ActiveModel {
                line_id: Set(line_id),
                option_index: Set(choice.option_index),
                option_text: Set(choice.option_text),
                target_scene_id: Set(Some(target_scene_id)),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(GalgameScriptRecordError::Database)?;
        }
    }

    Ok(script)
}
