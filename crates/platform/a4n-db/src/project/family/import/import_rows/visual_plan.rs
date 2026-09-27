use a4n_db_models::models::{chapter::chapter_visual_plan, project::source_snapshot_chapter};
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};

use crate::project::family::import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, map_optional, map_required,
    remap_chapter_source_revision_key, remap_visual_plan_identity,
};

pub(super) async fn insert_visual_plan_rows(
    user_id: i64,
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        &package.chapter_visual_plans,
        plan,
        chapter_visual_plan::ActiveModel,
        maps.visual_plan_ids,
        "导入章节视觉编排失败",
        |active| {
            let source_chapter_id = map_required(
                &maps.chapter_ids,
                plan.source_chapter_id,
                "章节视觉编排 source_chapter_id",
            )?;
            let source_snapshot_id = map_optional(
                &maps.source_snapshot_ids,
                plan.source_snapshot_id,
                "章节视觉编排 source_snapshot_id",
            )?;
            let source_snapshot_chapter_id = map_optional(
                &maps.source_snapshot_chapter_ids,
                plan.source_snapshot_chapter_id,
                "章节视觉编排 source_snapshot_chapter_id",
            )?;
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                plan.root_project_id,
                "章节视觉编排 root_project_id",
            )?);
            active.source_chapter_id = Set(source_chapter_id);
            active.source_snapshot_id = Set(source_snapshot_id);
            active.source_snapshot_chapter_id = Set(source_snapshot_chapter_id);
            active.source_revision_key = Set(remap_chapter_source_revision_key(
                &format!("章节视觉编排 #{}", plan.id),
                (
                    plan.source_chapter_id,
                    plan.source_snapshot_id,
                    plan.source_snapshot_chapter_id,
                ),
                (
                    source_chapter_id,
                    source_snapshot_id,
                    source_snapshot_chapter_id,
                ),
                &plan.source_content_sha256,
                &plan.source_revision_key,
            )?);
            active.author_user_id = Set(user_id);
        }
    );

    for plan in &package.chapter_visual_plans {
        maps.visual_plan_sha256s
            .insert(plan.id, plan.plan_sha256.clone());
    }

    let mut visual_plan_id_case = CaseStatement::new();
    let mut visual_plan_version_case = CaseStatement::new();
    let mut visual_plan_sha256_case = CaseStatement::new();
    let mut chapter_ids = Vec::new();
    for chapter in &package.source_snapshot_chapters {
        let (visual_plan_id, visual_plan_version, visual_plan_sha256) = remap_visual_plan_identity(
            chapter.visual_plan_id,
            chapter.visual_plan_version,
            chapter.visual_plan_sha256.as_deref(),
            maps,
            "源快照章节 visual plan",
        )?;
        let Some(visual_plan_id) = visual_plan_id else {
            continue;
        };
        let new_chapter_id = map_required(
            &maps.source_snapshot_chapter_ids,
            chapter.id,
            "源快照章节 visual plan 回填",
        )?;
        visual_plan_id_case = visual_plan_id_case.case(
            source_snapshot_chapter::Column::Id.eq(new_chapter_id),
            Expr::value(Some(visual_plan_id)),
        );
        visual_plan_version_case = visual_plan_version_case.case(
            source_snapshot_chapter::Column::Id.eq(new_chapter_id),
            Expr::value(visual_plan_version),
        );
        visual_plan_sha256_case = visual_plan_sha256_case.case(
            source_snapshot_chapter::Column::Id.eq(new_chapter_id),
            Expr::value(visual_plan_sha256),
        );
        chapter_ids.push(new_chapter_id);
    }
    if !chapter_ids.is_empty() {
        let updated = source_snapshot_chapter::Entity::update_many()
            .col_expr(
                source_snapshot_chapter::Column::VisualPlanId,
                visual_plan_id_case
                    .finally(Expr::col(source_snapshot_chapter::Column::VisualPlanId))
                    .into(),
            )
            .col_expr(
                source_snapshot_chapter::Column::VisualPlanVersion,
                visual_plan_version_case
                    .finally(Expr::col(
                        source_snapshot_chapter::Column::VisualPlanVersion,
                    ))
                    .into(),
            )
            .col_expr(
                source_snapshot_chapter::Column::VisualPlanSha256,
                visual_plan_sha256_case
                    .finally(Expr::col(source_snapshot_chapter::Column::VisualPlanSha256))
                    .into(),
            )
            .filter(source_snapshot_chapter::Column::Id.is_in(chapter_ids.clone()))
            .exec(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "回填源快照章节视觉编排身份失败",
                source,
            })?;
        if updated.rows_affected != chapter_ids.len() as u64 {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "源快照章节 visual plan 回填数量不一致: updated={}, expected={}",
                updated.rows_affected,
                chapter_ids.len()
            )));
        }
    }
    Ok(())
}

/// Image IDs are available only after image rows have been inserted. Finish all
/// director references and hashes before importing media that depends on scripts.
pub(super) async fn remap_visual_plan_resources(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    use a4n_db_models::models::chapter::chapter_shooting_script;
    use a4n_domain::{
        chapter_visual_plan::ChapterVisualPlanV3, shooting_script::ChapterShootingScriptV2,
    };
    for original in &package.chapter_visual_plans {
        let mut plan: ChapterVisualPlanV3 = serde_json::from_value(original.plan_json.clone())
            .map_err(|error| ProjectFamilyImportError::invalid_package(error.to_string()))?;
        plan.remap_resources(
            |id| {
                map_required(&maps.generated_image_ids, id, "导演图片引用")
                    .map_err(|error| error.to_string())
            },
            |id| {
                map_required(&maps.scene_profile_ids, id, "导演场景引用")
                    .map_err(|error| error.to_string())
            },
        )
        .map_err(ProjectFamilyImportError::invalid_package)?;
        let hash = crate::generation::canonical_json_sha256(&plan)
            .map_err(|error| ProjectFamilyImportError::invalid_package(error.to_string()))?;
        let id = map_required(&maps.visual_plan_ids, original.id, "导演编排")?;
        chapter_visual_plan::ActiveModel {
            id: Set(id),
            plan_json: Set(serde_json::to_value(&plan)
                .map_err(|error| ProjectFamilyImportError::invalid_package(error.to_string()))?),
            plan_sha256: Set(hash.clone()),
            ..Default::default()
        }
        .update(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "重映射导演编排资源失败",
            source,
        })?;
        source_snapshot_chapter::Entity::update_many()
            .col_expr(
                source_snapshot_chapter::Column::VisualPlanSha256,
                Expr::value(Some(hash.clone())),
            )
            .filter(source_snapshot_chapter::Column::VisualPlanId.eq(id))
            .exec(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "更新导演来源快照哈希失败",
                source,
            })?;
        maps.visual_plan_sha256s.insert(original.id, hash);
    }
    for original in &package.chapter_shooting_scripts {
        let mut script: ChapterShootingScriptV2 =
            serde_json::from_value(original.script_json.clone())
                .map_err(|error| ProjectFamilyImportError::invalid_package(error.to_string()))?;
        let director = original
            .visual_plan_id
            .map(|id| {
                let record = package
                    .chapter_visual_plans
                    .iter()
                    .find(|plan| plan.id == id)
                    .ok_or_else(|| {
                        ProjectFamilyImportError::invalid_package(
                            "拍摄剧本引用的导演编排不在导入包中",
                        )
                    })?;
                let plan: ChapterVisualPlanV3 = serde_json::from_value(record.plan_json.clone())
                    .map_err(|error| {
                        ProjectFamilyImportError::invalid_package(error.to_string())
                    })?;
                a4n_domain::shooting_script::project_visual_plan_for_shooting_script(&plan, &[])
                    .map_err(|error| ProjectFamilyImportError::invalid_package(error.to_string()))
            })
            .transpose()?;
        let fingerprints = |script: &ChapterShootingScriptV2| match &director {
            Some(director) => script
                .shot_content_fingerprints(director)
                .map_err(|error| ProjectFamilyImportError::invalid_package(error.to_string())),
            // 历史未绑定剧本保留原数据，但不能为它证明可执行的视频来源。
            None => Ok(Default::default()),
        };
        let original_shots = fingerprints(&script)?;
        for shot in script.scenes.iter_mut().flat_map(|scene| &mut scene.shots) {
            if let Some(constraints) = &mut shot.visual_constraints {
                constraints
                    .remap_resources(
                        |id| {
                            map_required(&maps.generated_image_ids, id, "拍摄剧本图片引用")
                                .map_err(|error| error.to_string())
                        },
                        |id| {
                            map_required(&maps.scene_profile_ids, id, "拍摄剧本场景引用")
                                .map_err(|error| error.to_string())
                        },
                    )
                    .map_err(ProjectFamilyImportError::invalid_package)?;
            }
        }
        let identity = remap_visual_plan_identity(
            original.visual_plan_id,
            original.visual_plan_version,
            original.visual_plan_sha256.as_deref(),
            maps,
            "拍摄剧本导演来源",
        )?;
        let remapped_shots = fingerprints(&script)?;
        maps.shooting_script_shot_fingerprints.insert(
            original.id,
            original_shots
                .into_iter()
                .map(|(id, hash)| {
                    let remapped = remapped_shots
                        .get(&id)
                        .expect("resource remapping preserves shot IDs")
                        .clone();
                    (id, (hash, remapped))
                })
                .collect(),
        );
        let (json, hash) =
            crate::project::family::import::remap_shooting_script_visual_plan_source(
                &serde_json::to_value(&script).map_err(|error| {
                    ProjectFamilyImportError::invalid_package(error.to_string())
                })?,
                (
                    original.visual_plan_id,
                    original.visual_plan_version,
                    original.visual_plan_sha256.as_deref(),
                ),
                (identity.0, identity.1, identity.2.as_deref()),
                "拍摄剧本导演资源",
            )?;
        chapter_shooting_script::ActiveModel {
            id: Set(map_required(
                &maps.shooting_script_ids,
                original.id,
                "拍摄剧本",
            )?),
            script_json: Set(json),
            script_sha256: Set(hash.clone()),
            visual_plan_sha256: Set(identity.2),
            ..Default::default()
        }
        .update(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "重映射拍摄剧本导演资源失败",
            source,
        })?;
        maps.shooting_script_sha256s.insert(original.id, hash);
    }
    Ok(())
}
