use a4n_db_models::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    source_sync::source_sync_run,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use crate::source_sync::SourceSyncDbError;
use crate::source_sync::galgame::{
    SourceSyncGalgameSceneWriteRecordInput, resolve_related_galgame_scene_conflict_records,
};

pub(super) struct PersistedGalgameSceneRegeneration {
    pub(super) new_line_count: usize,
    pub(super) resolved_related_conflict_count: u64,
}

pub(super) async fn persist_galgame_scene_regeneration_records<C>(
    db: &C,
    run: &source_sync_run::Model,
    conflict_id: i64,
    scene: galgame_scene::Model,
    script: galgame_script::Model,
    new_source_snapshot_chapter_id: i64,
    generated_scene: SourceSyncGalgameSceneWriteRecordInput,
    old_line_ids: &[i64],
) -> Result<PersistedGalgameSceneRegeneration, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let new_line_count = generated_scene.lines.len();
    let line_inserts = generated_scene
        .lines
        .into_iter()
        .map(|line| galgame_line::ActiveModel {
            scene_id: Set(scene.id),
            line_index: Set(line.line_index),
            line_type: Set(line.line_type),
            speaker_profile_id: Set(line.speaker_profile_id),
            text: Set(line.text),
            expression: Set(line.expression),
            pose: Set(line.pose),
            position: Set(line.position),
            presentation_json: Set(line.presentation_json),
            text_anchor_id: Set(line.text_anchor_id),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    if !line_inserts.is_empty() {
        galgame_line::Entity::insert_many(line_inserts)
            .exec_without_returning(db)
            .await?;
    }

    let mut active_scene: galgame_scene::ActiveModel = scene.clone().into();
    active_scene.source_snapshot_chapter_id = Set(new_source_snapshot_chapter_id);
    active_scene.title = Set(generated_scene.title);
    active_scene.location = Set(generated_scene.location);
    active_scene.time_of_day = Set(generated_scene.time_of_day);
    active_scene.participants_json = Set(generated_scene.participants_json);
    active_scene.background_note = Set(generated_scene.background_note);
    active_scene.updated_at = Set(now);
    active_scene.update(db).await?;

    let mut active_script: galgame_script::ActiveModel = script.into();
    active_script.updated_at = Set(now);
    active_script.update(db).await?;

    let resolved_related_conflict_count = resolve_related_galgame_scene_conflict_records(
        db,
        run.id,
        conflict_id,
        scene.id,
        old_line_ids,
    )
    .await?;

    Ok(PersistedGalgameSceneRegeneration {
        new_line_count,
        resolved_related_conflict_count,
    })
}
