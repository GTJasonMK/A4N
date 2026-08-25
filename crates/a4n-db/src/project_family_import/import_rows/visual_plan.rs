use a4n_core::models::{chapter::chapter_visual_plan, project::source_snapshot_chapter};
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};

use crate::project_family_import::{
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
