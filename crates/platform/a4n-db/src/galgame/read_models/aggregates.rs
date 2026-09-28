use crate::galgame::{
    GalgameAssetPlanningReadModel, GalgameAssetPlanningSceneRecord,
    GalgameAssetPlanningSourceChapterRecord, GalgamePipelinePromptSpeakerProfileRecord,
    GalgamePipelinePromptTextAnchorRecord, GalgameScriptDetailChoiceRecord,
    GalgameScriptDetailReadModel, GalgameScriptDetailSceneRecord, GalgameScriptDetailScriptRecord,
    galgame_media_binding_read_model,
};
use a4n_db_models::models::{
    character::protagonist_profile,
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    project::{source_snapshot_chapter, text_anchor},
};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use std::collections::HashSet;

pub async fn galgame_asset_planning_read_model(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Option<GalgameAssetPlanningReadModel>, DbErr> {
    let Some((scenes, lines)) = galgame_script_scene_lines(db, project_id).await? else {
        return Ok(None);
    };
    let source_snapshot_chapter_ids = scenes
        .iter()
        .map(|scene| scene.source_snapshot_chapter_id)
        .collect::<HashSet<_>>();
    let source_snapshot_chapters = if source_snapshot_chapter_ids.is_empty() {
        Vec::new()
    } else {
        source_snapshot_chapter::Entity::find()
            .select_only()
            .column(source_snapshot_chapter::Column::Id)
            .column(source_snapshot_chapter::Column::SnapshotId)
            .column(source_snapshot_chapter::Column::SourceChapterId)
            .column(source_snapshot_chapter::Column::ChapterNumber)
            .column(source_snapshot_chapter::Column::ContentHash)
            .filter(source_snapshot_chapter::Column::Id.is_in(source_snapshot_chapter_ids))
            .into_model::<GalgameAssetPlanningSourceChapterRecord>()
            .all(db)
            .await?
    };
    Ok(Some(GalgameAssetPlanningReadModel {
        scenes,
        lines,
        source_snapshot_chapters,
    }))
}

pub async fn galgame_script_detail_read_model(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Option<GalgameScriptDetailReadModel>, DbErr> {
    let Some((script, scenes, lines)) = galgame_script_detail_scene_lines(db, project_id).await?
    else {
        return Ok(None);
    };
    let scene_ids = scenes.iter().map(|scene| scene.id).collect::<Vec<_>>();
    let line_ids = lines.iter().map(|line| line.id).collect::<Vec<_>>();
    let text_anchor_ids = lines
        .iter()
        .filter_map(|line| line.text_anchor_id)
        .collect::<HashSet<_>>();
    // These projections depend only on the already loaded IDs. Overlap their pool waits, then
    // unwrap in the old order so the first reported error remains deterministic.
    let (choices_result, text_anchors_result, speaker_profiles_result, media_bindings_result) = futures::join!(
        async {
            if line_ids.is_empty() {
                Ok(Vec::new())
            } else {
                galgame_choice::Entity::find()
                    .select_only()
                    .column(galgame_choice::Column::Id)
                    .column(galgame_choice::Column::LineId)
                    .column(galgame_choice::Column::OptionIndex)
                    .column(galgame_choice::Column::OptionText)
                    .column(galgame_choice::Column::TargetSceneId)
                    .filter(galgame_choice::Column::LineId.is_in(line_ids.clone()))
                    .order_by_asc(galgame_choice::Column::LineId)
                    .order_by_asc(galgame_choice::Column::OptionIndex)
                    .into_model::<GalgameScriptDetailChoiceRecord>()
                    .all(db)
                    .await
            }
        },
        async {
            if text_anchor_ids.is_empty() {
                Ok(Vec::new())
            } else {
                text_anchor::Entity::find()
                    .select_only()
                    .column(text_anchor::Column::Id)
                    .column(text_anchor::Column::SourceSnapshotChapterId)
                    .column(text_anchor::Column::ParagraphIndex)
                    .column(text_anchor::Column::StartOffset)
                    .column(text_anchor::Column::EndOffset)
                    .column(text_anchor::Column::Quote)
                    .filter(text_anchor::Column::Id.is_in(text_anchor_ids.clone()))
                    .filter(text_anchor::Column::ProjectId.eq(script.root_project_id))
                    .filter(
                        text_anchor::Column::SourceSnapshotChapterId
                            .is_in(scenes.iter().map(|scene| scene.source_snapshot_chapter_id)),
                    )
                    .order_by_asc(text_anchor::Column::Id)
                    .into_model::<GalgamePipelinePromptTextAnchorRecord>()
                    .all(db)
                    .await
            }
        },
        async {
            protagonist_profile::Entity::find()
                .select_only()
                .column(protagonist_profile::Column::Id)
                .column(protagonist_profile::Column::Name)
                .filter(protagonist_profile::Column::ProjectId.eq(script.root_project_id))
                .order_by_asc(protagonist_profile::Column::Id)
                .into_model::<GalgamePipelinePromptSpeakerProfileRecord>()
                .all(db)
                .await
        },
        async { galgame_media_binding_read_model(db, &scene_ids, &line_ids).await },
    );
    let choices = choices_result?;
    let text_anchors = text_anchors_result?;
    let speaker_profiles = speaker_profiles_result?;
    let media_bindings = media_bindings_result?;
    Ok(Some(GalgameScriptDetailReadModel {
        script,
        speaker_profiles,
        text_anchors,
        media_bindings,
        scenes,
        lines,
        choices,
    }))
}

async fn galgame_script_detail_scene_lines(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<
    Option<(
        GalgameScriptDetailScriptRecord,
        Vec<GalgameScriptDetailSceneRecord>,
        Vec<galgame_line::Model>,
    )>,
    DbErr,
> {
    let Some(script) = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .column(galgame_script::Column::RootProjectId)
        .column(galgame_script::Column::Title)
        .filter(galgame_script::Column::ProjectId.eq(project_id))
        .into_model::<GalgameScriptDetailScriptRecord>()
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let scenes = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .column(galgame_scene::Column::SourceSnapshotChapterId)
        .column(galgame_scene::Column::AdaptationPlanId)
        .column(galgame_scene::Column::AdaptationUnitId)
        .column(galgame_scene::Column::SceneIndex)
        .column(galgame_scene::Column::Title)
        .column(galgame_scene::Column::Location)
        .column(galgame_scene::Column::TimeOfDay)
        .column(galgame_scene::Column::BackgroundNote)
        .column(galgame_scene::Column::EntryKind)
        .column(galgame_scene::Column::NextSceneId)
        .filter(galgame_scene::Column::ScriptId.eq(script.id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .into_model::<GalgameScriptDetailSceneRecord>()
        .all(db)
        .await?;
    let scene_ids = scenes.iter().map(|scene| scene.id).collect::<Vec<_>>();
    let lines = if scene_ids.is_empty() {
        Vec::new()
    } else {
        galgame_line::Entity::find()
            .filter(galgame_line::Column::SceneId.is_in(scene_ids))
            .order_by_asc(galgame_line::Column::SceneId)
            .order_by_asc(galgame_line::Column::LineIndex)
            .all(db)
            .await?
    };
    Ok(Some((script, scenes, lines)))
}

async fn galgame_script_scene_lines(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<
    Option<(
        Vec<GalgameAssetPlanningSceneRecord>,
        Vec<galgame_line::Model>,
    )>,
    DbErr,
> {
    let Some(script_id) = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .filter(galgame_script::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let scenes = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .column(galgame_scene::Column::SourceSnapshotChapterId)
        .column(galgame_scene::Column::SceneIndex)
        .column(galgame_scene::Column::Title)
        .column(galgame_scene::Column::Location)
        .column(galgame_scene::Column::TimeOfDay)
        .column(galgame_scene::Column::BackgroundNote)
        .column(galgame_scene::Column::EntryKind)
        .column(galgame_scene::Column::NextSceneId)
        .filter(galgame_scene::Column::ScriptId.eq(script_id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .into_model::<GalgameAssetPlanningSceneRecord>()
        .all(db)
        .await?;
    let scene_ids = scenes.iter().map(|scene| scene.id).collect::<Vec<_>>();
    let lines = if scene_ids.is_empty() {
        Vec::new()
    } else {
        galgame_line::Entity::find()
            .filter(galgame_line::Column::SceneId.is_in(scene_ids))
            .order_by_asc(galgame_line::Column::SceneId)
            .order_by_asc(galgame_line::Column::LineIndex)
            .all(db)
            .await?
    };
    Ok(Some((scenes, lines)))
}
