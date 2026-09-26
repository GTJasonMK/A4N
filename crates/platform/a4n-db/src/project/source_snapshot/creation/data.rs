use a4n_db_models::models::{
    chapter::{chapter, chapter_shooting_script, chapter_visual_plan},
    project::blueprint_version,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::json;

use super::rules::chapter_scope_json;
use crate::project::{
    ProjectMutationError,
    source_snapshot::{
        DerivedChapterScope,
        hashing::{hash_json, hash_text, shooting_script_source_hash, to_json},
        profiles::load_snapshot_profiles,
        types::{SourceSnapshotBuildData, SourceSnapshotShootingScript},
    },
};
use std::collections::{HashMap, HashSet};

pub(crate) async fn source_snapshot_version_hash<C: ConnectionTrait>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<String, ProjectMutationError> {
    Ok(build_source_snapshot_data(db, root_project_id, scope)
        .await?
        .source_version_hash)
}

pub(super) async fn build_source_snapshot_data<C>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<SourceSnapshotBuildData, ProjectMutationError>
where
    C: ConnectionTrait,
{
    let chapters = chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(root_project_id))
        .filter(chapter::Column::ChapterNumber.gte(scope.start_chapter))
        .filter(chapter::Column::ChapterNumber.lte(scope.end_chapter))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?;
    if chapters.is_empty() {
        return Err(ProjectMutationError::InvalidArgument(
            "源文本快照范围内没有章节".to_string(),
        ));
    }
    let expected_numbers = (scope.start_chapter..=scope.end_chapter).collect::<Vec<_>>();
    let actual_numbers = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .collect::<Vec<_>>();
    if actual_numbers != expected_numbers {
        let actual_number_set = actual_numbers.iter().copied().collect::<HashSet<_>>();
        let missing = expected_numbers
            .into_iter()
            .filter(|number| !actual_number_set.contains(number))
            .collect::<Vec<_>>();
        return Err(ProjectMutationError::InvalidArgument(format!(
            "源文本快照范围内缺少章节: {:?}",
            missing
        )));
    }

    let content_hashes = chapters
        .iter()
        .map(|chapter| (chapter.id, hash_text(&chapter.content)))
        .collect::<HashMap<_, _>>();
    let visual_plans = chapter_visual_plan::Entity::find()
        .filter(chapter_visual_plan::Column::RootProjectId.eq(root_project_id))
        .filter(chapter_visual_plan::Column::SourceChapterId.is_in(content_hashes.keys().copied()))
        .filter(chapter_visual_plan::Column::SourceSnapshotId.is_null())
        .filter(chapter_visual_plan::Column::SourceSnapshotChapterId.is_null())
        .filter(chapter_visual_plan::Column::Status.eq("confirmed"))
        .distinct_on([
            chapter_visual_plan::Column::SourceChapterId,
            chapter_visual_plan::Column::SourceContentSha256,
        ])
        .order_by_asc(chapter_visual_plan::Column::SourceChapterId)
        .order_by_asc(chapter_visual_plan::Column::SourceContentSha256)
        .order_by_desc(chapter_visual_plan::Column::UpdatedAt)
        .order_by_desc(chapter_visual_plan::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .filter(|plan| {
            content_hashes.get(&plan.source_chapter_id) == Some(&plan.source_content_sha256)
        })
        .map(|plan| (plan.source_chapter_id, plan))
        .collect::<HashMap<_, _>>();
    let mut shooting_scripts = chapter_shooting_script::Entity::find()
        .filter(chapter_shooting_script::Column::RootProjectId.eq(root_project_id))
        .filter(
            chapter_shooting_script::Column::SourceChapterId.is_in(content_hashes.keys().copied()),
        )
        .filter(chapter_shooting_script::Column::SourceSnapshotId.is_null())
        .filter(chapter_shooting_script::Column::SourceSnapshotChapterId.is_null())
        .distinct_on([
            chapter_shooting_script::Column::SourceChapterId,
            chapter_shooting_script::Column::SourceContentSha256,
            chapter_shooting_script::Column::VisualPlanId,
            chapter_shooting_script::Column::VisualPlanVersion,
            chapter_shooting_script::Column::VisualPlanSha256,
        ])
        .order_by_asc(chapter_shooting_script::Column::SourceChapterId)
        .order_by_asc(chapter_shooting_script::Column::SourceContentSha256)
        .order_by_asc(chapter_shooting_script::Column::VisualPlanId)
        .order_by_asc(chapter_shooting_script::Column::VisualPlanVersion)
        .order_by_asc(chapter_shooting_script::Column::VisualPlanSha256)
        .order_by_desc(chapter_shooting_script::Column::UpdatedAt)
        .order_by_desc(chapter_shooting_script::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .filter(|script| {
            visual_plans
                .get(&script.source_chapter_id)
                .is_some_and(|plan| {
                    script.source_content_sha256 == plan.source_content_sha256
                        && script.visual_plan_id == Some(plan.id)
                        && script.visual_plan_version == Some(plan.plan_version)
                        && script.visual_plan_sha256.as_ref() == Some(&plan.plan_sha256)
                })
        })
        .map(|script| {
            let script = crate::shooting_script::parse_record(script)?.record;
            Ok((
                script.source_chapter_id,
                SourceSnapshotShootingScript {
                    script_json: script.script_json,
                    input_fingerprint_sha256: script.input_fingerprint_sha256,
                },
            ))
        })
        .collect::<Result<HashMap<_, _>, ProjectMutationError>>()?;
    let incomplete_plans = visual_plans
        .values()
        .filter(|plan| !shooting_scripts.contains_key(&plan.source_chapter_id))
        .collect::<Vec<_>>();
    for (chapter_id, (input_fingerprint_sha256, mut progress)) in
        crate::shooting_script::chapter_shooting_script_progress_for_plans(
            db,
            root_project_id,
            &incomplete_plans,
        )
        .await?
    {
        if !progress.script.scenes.is_empty() {
            // 冻结当前已完成的场景子图，后续同步再引入尚未生成场景及其转场。
            let present = progress
                .script
                .scenes
                .iter()
                .map(|scene| scene.narrative_unit_index)
                .collect::<HashSet<_>>();
            for scene in &mut progress.script.scenes {
                scene
                    .transition_contracts
                    .retain(|transition| present.contains(&transition.to_narrative_unit_index));
            }
            shooting_scripts.insert(
                chapter_id,
                SourceSnapshotShootingScript {
                    script_json: to_json(&progress.script)?,
                    input_fingerprint_sha256,
                },
            );
        }
    }
    let chapter_hashes_json = chapters
        .iter()
        .map(|chapter| {
            let mut hashes = json!({
                "source_chapter_id": chapter.id,
                "chapter_number": chapter.chapter_number,
                "content_hash": content_hashes[&chapter.id],
                "analysis_hash": chapter.analysis_data.as_ref().map(hash_json),
            });
            if let Some(plan) = visual_plans.get(&chapter.id) {
                hashes["visual_plan_hash"] = json!(plan.plan_sha256);
            }
            if let Some(script) = shooting_scripts.get(&chapter.id) {
                hashes["shooting_script_hash"] =
                    json!(shooting_script_source_hash(&script.script_json));
            }
            hashes
        })
        .collect::<Vec<_>>();
    let profiles = load_snapshot_profiles(db, root_project_id).await?;
    let profile_hashes_json = json!(
        profiles
            .iter()
            .map(|profile| json!({
                "kind": profile.kind,
                "id": profile.id,
                "hash": hash_json(&profile.payload),
            }))
            .collect::<Vec<_>>()
    );
    let latest_blueprint_version_id = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(root_project_id))
        .order_by_desc(blueprint_version::Column::CreatedAt)
        .order_by_desc(blueprint_version::Column::Id)
        .one(db)
        .await
        .map_err(ProjectMutationError::Database)?
        .map(|version| version.id);
    let chapter_scope_json = chapter_scope_json(scope);
    let source_version_hash = hash_json(&json!({
        "root_project_id": root_project_id,
        "chapter_scope": chapter_scope_json,
        "chapter_hashes": chapter_hashes_json,
        "profile_hashes": profile_hashes_json,
        "blueprint_version_id": latest_blueprint_version_id,
    }));

    Ok(SourceSnapshotBuildData {
        chapters,
        visual_plans,
        shooting_scripts,
        profiles,
        chapter_hashes_json,
        profile_hashes_json,
        latest_blueprint_version_id,
        chapter_scope_json,
        source_version_hash,
    })
}
