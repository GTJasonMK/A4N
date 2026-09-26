use a4n_db_models::models::{
    asset::object_profile,
    chapter::{
        chapter, chapter_outline, chapter_shooting_script, chapter_visual_plan, part_outline,
    },
    character::protagonist_profile,
    project::{blueprint_version, novel_blueprint},
    scene::scene_profile,
    user::conversation,
};
use a4n_error::AppError;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::{
    BlueprintGraphRow, BlueprintVersionGraphRow, ChapterGraphRow, ChapterOutlineGraphRow,
    CharacterProfileGraphRow, ConversationGraphRow, ObjectProfileGraphRow, PartOutlineGraphRow,
    SceneProfileGraphRow, ShootingScriptGraphRow, VisualPlanGraphRow,
};

pub(super) struct RootAssetGraphRecords {
    pub(super) conversations: Vec<ConversationGraphRow>,
    pub(super) blueprints: Vec<BlueprintGraphRow>,
    pub(super) blueprint_versions: Vec<BlueprintVersionGraphRow>,
    pub(super) part_outlines: Vec<PartOutlineGraphRow>,
    pub(super) chapter_outlines: Vec<ChapterOutlineGraphRow>,
    pub(super) chapters: Vec<ChapterGraphRow>,
    pub(super) visual_plans: Vec<VisualPlanGraphRow>,
    pub(super) shooting_scripts: Vec<ShootingScriptGraphRow>,
    pub(super) character_profiles: Vec<CharacterProfileGraphRow>,
    pub(super) object_profiles: Vec<ObjectProfileGraphRow>,
    pub(super) scene_profiles: Vec<SceneProfileGraphRow>,
}

pub(super) async fn load_root_asset_graph_records(
    db: &impl ConnectionTrait,
    root_project_id: i64,
) -> Result<RootAssetGraphRecords, AppError> {
    // Every root projection is read-only and independent. Overlap pool waits, then extract
    // results in the original order so the first reported database error remains stable.
    let (
        conversations_result,
        blueprints_result,
        blueprint_versions_result,
        part_outlines_result,
        chapter_outlines_result,
        chapters_result,
        visual_plans_result,
        shooting_scripts_result,
        character_profiles_result,
        object_profiles_result,
        scene_profiles_result,
    ) = futures::join!(
        conversation::Entity::find()
            .select_only()
            .column(conversation::Column::MessageCount)
            .filter(conversation::Column::ProjectId.eq(root_project_id))
            .order_by_asc(conversation::Column::Id)
            .into_model::<ConversationGraphRow>()
            .all(db),
        novel_blueprint::Entity::find()
            .select_only()
            .column(novel_blueprint::Column::CurrentVersion)
            .column(novel_blueprint::Column::OneSentenceSummary)
            .filter(novel_blueprint::Column::ProjectId.eq(root_project_id))
            .order_by_asc(novel_blueprint::Column::Id)
            .into_model::<BlueprintGraphRow>()
            .all(db),
        blueprint_version::Entity::find()
            .select_only()
            .column(blueprint_version::Column::Version)
            .filter(blueprint_version::Column::ProjectId.eq(root_project_id))
            .order_by_asc(blueprint_version::Column::Id)
            .into_model::<BlueprintVersionGraphRow>()
            .all(db),
        part_outline::Entity::find()
            .select_only()
            .column(part_outline::Column::StartChapter)
            .column(part_outline::Column::EndChapter)
            .filter(part_outline::Column::ProjectId.eq(root_project_id))
            .order_by_asc(part_outline::Column::PartNumber)
            .into_model::<PartOutlineGraphRow>()
            .all(db),
        chapter_outline::Entity::find()
            .select_only()
            .column(chapter_outline::Column::ChapterNumber)
            .filter(chapter_outline::Column::ProjectId.eq(root_project_id))
            .order_by_asc(chapter_outline::Column::ChapterNumber)
            .into_model::<ChapterOutlineGraphRow>()
            .all(db),
        chapter::Entity::find()
            .select_only()
            .column(chapter::Column::Id)
            .column(chapter::Column::Content)
            .column(chapter::Column::WordCount)
            .column_as(
                Expr::col(chapter::Column::AnalysisData).is_not_null().and(
                    Expr::col(chapter::Column::AnalysisData)
                        .cast_as("text")
                        .ne("null"),
                ),
                "has_analysis",
            )
            .filter(chapter::Column::ProjectId.eq(root_project_id))
            .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
            .order_by_asc(chapter::Column::ChapterNumber)
            .into_model::<ChapterGraphRow>()
            .all(db),
        chapter_visual_plan::Entity::find()
            .select_only()
            .column(chapter_visual_plan::Column::SourceChapterId)
            .column(chapter_visual_plan::Column::SourceSnapshotId)
            .column(chapter_visual_plan::Column::SourceContentSha256)
            .column(chapter_visual_plan::Column::Status)
            .filter(chapter_visual_plan::Column::RootProjectId.eq(root_project_id))
            .order_by_asc(chapter_visual_plan::Column::SourceChapterId)
            .order_by_asc(chapter_visual_plan::Column::PlanVersion)
            .into_model::<VisualPlanGraphRow>()
            .all(db),
        chapter_shooting_script::Entity::find()
            .select_only()
            .column(chapter_shooting_script::Column::SourceChapterId)
            .column(chapter_shooting_script::Column::SourceSnapshotId)
            .column(chapter_shooting_script::Column::SourceContentSha256)
            .filter(chapter_shooting_script::Column::RootProjectId.eq(root_project_id))
            .order_by_asc(chapter_shooting_script::Column::SourceChapterId)
            .into_model::<ShootingScriptGraphRow>()
            .all(db),
        protagonist_profile::Entity::find()
            .select_only()
            .column(protagonist_profile::Column::Id)
            .column(protagonist_profile::Column::Name)
            .column(protagonist_profile::Column::BaseAttributes)
            .column(protagonist_profile::Column::Appearance)
            .column(protagonist_profile::Column::LastSyncedChapter)
            .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
            .order_by_asc(protagonist_profile::Column::Id)
            .into_model::<CharacterProfileGraphRow>()
            .all(db),
        object_profile::Entity::find()
            .select_only()
            .column(object_profile::Column::Id)
            .column(object_profile::Column::Name)
            .column(object_profile::Column::VisualDescription)
            .column(object_profile::Column::FirstSeenChapter)
            .column(object_profile::Column::LastSeenChapter)
            .column(object_profile::Column::Importance)
            .filter(object_profile::Column::ProjectId.eq(root_project_id))
            .order_by_asc(object_profile::Column::Id)
            .into_model::<ObjectProfileGraphRow>()
            .all(db),
        scene_profile::Entity::find()
            .select_only()
            .column(scene_profile::Column::Id)
            .column(scene_profile::Column::Name)
            .column(scene_profile::Column::VisualDescription)
            .column(scene_profile::Column::FirstSeenChapter)
            .column(scene_profile::Column::LastSeenChapter)
            .column(scene_profile::Column::Importance)
            .filter(scene_profile::Column::ProjectId.eq(root_project_id))
            .order_by_asc(scene_profile::Column::Id)
            .into_model::<SceneProfileGraphRow>()
            .all(db),
    );
    let conversations = conversations_result.map_err(AppError::database)?;
    let blueprints = blueprints_result.map_err(AppError::database)?;
    let blueprint_versions = blueprint_versions_result.map_err(AppError::database)?;
    let part_outlines = part_outlines_result.map_err(AppError::database)?;
    let chapter_outlines = chapter_outlines_result.map_err(AppError::database)?;
    let chapters = chapters_result.map_err(AppError::database)?;
    let content_hashes = chapters
        .iter()
        .map(|chapter| (chapter.id, a4n_domain::source_text_hash(&chapter.content)))
        .collect::<std::collections::HashMap<_, _>>();
    let visual_plans = visual_plans_result
        .map_err(AppError::database)?
        .into_iter()
        .filter(|plan| {
            plan.source_snapshot_id.is_some()
                || content_hashes
                    .get(&plan.source_chapter_id)
                    .is_some_and(|hash| hash == &plan.source_content_sha256)
        })
        .collect::<Vec<_>>();
    let shooting_scripts = shooting_scripts_result
        .map_err(AppError::database)?
        .into_iter()
        .filter(|script| {
            script.source_snapshot_id.is_some()
                || content_hashes
                    .get(&script.source_chapter_id)
                    .is_some_and(|hash| hash == &script.source_content_sha256)
        })
        .collect::<Vec<_>>();
    let character_profiles = character_profiles_result.map_err(AppError::database)?;
    let object_profiles = object_profiles_result.map_err(AppError::database)?;
    let scene_profiles = scene_profiles_result.map_err(AppError::database)?;

    Ok(RootAssetGraphRecords {
        conversations,
        blueprints,
        blueprint_versions,
        part_outlines,
        chapter_outlines,
        chapters,
        visual_plans,
        shooting_scripts,
        character_profiles,
        object_profiles,
        scene_profiles,
    })
}
