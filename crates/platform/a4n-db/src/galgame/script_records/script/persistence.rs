use crate::galgame::{
    GALGAME_SCENE_ENTRY_BRANCH, GALGAME_SCENE_ENTRY_MAIN, GalgameScriptRecordError,
    PersistGalgameScriptRecordInput,
};
use a4n_db_models::models::galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script};
use chrono::Utc;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;

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

    let mut scene_rows = Vec::with_capacity(input.scenes.len());
    let mut scene_lines = Vec::with_capacity(input.scenes.len());
    let mut pending_scene_next = Vec::new();
    for scene_input in input.scenes {
        let crate::galgame::PersistGalgameSceneRecordInput {
            scene_index,
            source_snapshot_chapter_id,
            shooting_scene_identity,
            title,
            location,
            time_of_day,
            participants_json,
            background_note,
            entry_kind,
            next_scene_index,
            lines,
        } = scene_input;
        scene_rows.push(galgame_scene::ActiveModel {
            script_id: Set(script.id),
            source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
            shooting_scene_identity: Set(shooting_scene_identity),
            scene_index: Set(scene_index),
            title: Set(title),
            location: Set(location),
            time_of_day: Set(time_of_day),
            participants_json: Set(participants_json),
            background_note: Set(background_note),
            entry_kind: Set(entry_kind),
            next_scene_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        });
        scene_lines.push((scene_index, lines));
        if let Some(next_scene_index) = next_scene_index {
            pending_scene_next.push((scene_index, next_scene_index));
        }
    }

    let inserted_scenes = galgame_scene::Entity::insert_many(scene_rows)
        .exec_with_returning_many(db)
        .await
        .map_err(GalgameScriptRecordError::Database)?;
    let scene_id_by_index = inserted_scenes
        .iter()
        .map(|scene| (scene.scene_index, scene.id))
        .collect::<HashMap<_, _>>();
    let scene_entry_kind_by_index = inserted_scenes
        .iter()
        .map(|scene| (scene.scene_index, scene.entry_kind.as_str()))
        .collect::<HashMap<_, _>>();

    let mut line_rows = Vec::new();
    let mut pending_choices = Vec::new();
    for (scene_index, lines) in scene_lines {
        let scene_id = scene_id_by_index
            .get(&scene_index)
            .copied()
            .ok_or_else(|| {
                GalgameScriptRecordError::InvalidArgument(format!(
                    "Galgame scene_index 插入后无法回读: {scene_index}"
                ))
            })?;
        for line_input in lines {
            let crate::galgame::PersistGalgameLineRecordInput {
                line_index,
                line_type,
                speaker_profile_id,
                text,
                expression,
                pose,
                position,
                presentation_json,
                text_anchor_id,
                choices,
            } = line_input;
            line_rows.push(galgame_line::ActiveModel {
                scene_id: Set(scene_id),
                line_index: Set(line_index),
                line_type: Set(line_type),
                speaker_profile_id: Set(speaker_profile_id),
                text: Set(text),
                expression: Set(expression),
                pose: Set(pose),
                position: Set(position),
                presentation_json: Set(presentation_json),
                text_anchor_id: Set(text_anchor_id),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            });
            if !choices.is_empty() {
                pending_choices.push((scene_id, line_index, choices));
            }
        }
    }

    let inserted_lines = if line_rows.is_empty() {
        Vec::new()
    } else {
        galgame_line::Entity::insert_many(line_rows)
            .exec_with_returning_many(db)
            .await
            .map_err(GalgameScriptRecordError::Database)?
    };
    let line_id_by_key = inserted_lines
        .into_iter()
        .map(|line| ((line.scene_id, line.line_index), line.id))
        .collect::<HashMap<_, _>>();

    if !pending_scene_next.is_empty() {
        let mut next_scene_case = CaseStatement::new();
        let mut scene_ids = Vec::with_capacity(pending_scene_next.len());
        for (scene_index, next_scene_index) in pending_scene_next {
            let scene_id = scene_id_by_index
                .get(&scene_index)
                .copied()
                .ok_or_else(|| {
                    GalgameScriptRecordError::InvalidArgument(format!(
                        "Galgame scene_index 插入后无法回读: {scene_index}"
                    ))
                })?;
            let next_scene_id = scene_id_by_index
                .get(&next_scene_index)
                .copied()
                .ok_or_else(|| {
                    GalgameScriptRecordError::InvalidArgument(format!(
                        "Galgame scene next_scene_index 不存在: scene_index={scene_index}, next_scene_index={next_scene_index}"
                    ))
                })?;
            if scene_entry_kind_by_index.get(&scene_index).copied()
                == Some(GALGAME_SCENE_ENTRY_BRANCH)
                && scene_entry_kind_by_index.get(&next_scene_index).copied()
                    != Some(GALGAME_SCENE_ENTRY_MAIN)
            {
                return Err(GalgameScriptRecordError::InvalidArgument(format!(
                    "Galgame 支线场 next_scene_index 必须指向主线场: scene_index={scene_index}, next_scene_index={next_scene_index}"
                )));
            }
            next_scene_case = next_scene_case.case(
                galgame_scene::Column::Id.eq(scene_id),
                Expr::value(next_scene_id),
            );
            scene_ids.push(scene_id);
        }
        galgame_scene::Entity::update_many()
            .col_expr(
                galgame_scene::Column::NextSceneId,
                next_scene_case
                    .finally(Expr::col(galgame_scene::Column::NextSceneId))
                    .into(),
            )
            .filter(galgame_scene::Column::Id.is_in(scene_ids))
            .exec(db)
            .await
            .map_err(GalgameScriptRecordError::Database)?;
    }

    let mut choice_rows = Vec::new();
    for (scene_id, line_index, choices) in pending_choices {
        let line_id = line_id_by_key
            .get(&(scene_id, line_index))
            .copied()
            .ok_or_else(|| {
                GalgameScriptRecordError::InvalidArgument(format!(
                    "Galgame line_index 插入后无法回读: scene_id={scene_id}, line_index={line_index}"
                ))
            })?;
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
            choice_rows.push(galgame_choice::ActiveModel {
                line_id: Set(line_id),
                option_index: Set(choice.option_index),
                option_text: Set(choice.option_text),
                target_scene_id: Set(Some(target_scene_id)),
                created_at: Set(now),
                ..Default::default()
            });
        }
    }
    if !choice_rows.is_empty() {
        galgame_choice::Entity::insert_many(choice_rows)
            .exec_without_returning(db)
            .await
            .map_err(GalgameScriptRecordError::Database)?;
    }

    Ok(script)
}
