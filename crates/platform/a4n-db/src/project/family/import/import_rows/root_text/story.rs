use a4n_db_models::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::{
        blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
        novel_project,
    },
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set,
    sea_query::Expr,
};

use crate::project::family::import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, map_optional,
};

pub(super) async fn insert_story_rows(
    txn: &DatabaseTransaction,
    root_project_id: i64,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let canonical_raw_blueprint = canonical_package_blueprint_snapshot(package)?;
    for version in &package.text_core.blueprint_versions {
        crate::project::validate_blueprint_version_snapshot(&version.blueprint_data).map_err(
            |error| {
                ProjectFamilyImportError::invalid_package(format!(
                    "项目族导入包蓝图版本快照无效: blueprint_version_id={}: {error}",
                    version.id
                ))
            },
        )?;
    }
    if let Some(blueprint) = &package.text_core.blueprint {
        insert_mapped_import_row!(
            txn,
            blueprint,
            novel_blueprint::ActiveModel,
            maps.blueprint_ids,
            "导入蓝图失败",
            |active| {
                active.project_id = Set(root_project_id);
            }
        );
    }
    insert_project_scoped_mapped_import_rows!(
        txn,
        &package.text_core.blueprint_versions,
        version,
        blueprint_version::ActiveModel,
        maps.blueprint_version_ids,
        "导入蓝图版本失败",
        root_project_id
    );
    insert_project_scoped_import_rows!(
        txn,
        &package.text_core.blueprint_characters,
        character,
        blueprint_character::ActiveModel,
        "导入蓝图角色失败",
        root_project_id
    );
    insert_project_scoped_import_rows!(
        txn,
        &package.text_core.blueprint_relationships,
        relationship,
        blueprint_relationship::ActiveModel,
        "导入蓝图关系失败",
        root_project_id
    );
    insert_project_scoped_mapped_import_rows!(
        txn,
        &package.text_core.part_outlines,
        part,
        part_outline::ActiveModel,
        maps.part_outline_ids,
        "导入分部大纲失败",
        root_project_id
    );
    insert_mapped_import_rows!(
        txn,
        &package.text_core.chapter_outlines,
        outline,
        chapter_outline::ActiveModel,
        maps.chapter_outline_ids,
        "导入章节大纲失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.part_id = Set(map_optional(
                &maps.part_outline_ids,
                outline.part_id,
                "章节大纲 part_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.text_core.chapters,
        chapter,
        chapter::ActiveModel,
        maps.chapter_ids,
        "导入章节失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.outline_id = Set(map_optional(
                &maps.chapter_outline_ids,
                chapter.outline_id,
                "章节 outline_id",
            )?);
        }
    );
    let updated = novel_project::Entity::update_many()
        .col_expr(
            novel_project::Column::RawBlueprint,
            Expr::value(canonical_raw_blueprint),
        )
        .filter(novel_project::Column::Id.eq(root_project_id))
        .exec(txn)
        .await?;
    if updated.rows_affected != 1 {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "项目族导入目标根项目不存在: {root_project_id}"
        )));
    }
    Ok(())
}

fn canonical_package_blueprint_snapshot(
    package: &ProjectFamilyImportPackage,
) -> Result<Option<String>, ProjectFamilyImportError> {
    let Some(blueprint) = &package.text_core.blueprint else {
        if package.text_core.blueprint_versions.is_empty()
            && package.text_core.blueprint_characters.is_empty()
            && package.text_core.blueprint_relationships.is_empty()
        {
            return Ok(None);
        }
        return Err(ProjectFamilyImportError::invalid_package(
            "项目族导入包缺少当前蓝图，但包含蓝图版本、角色或关系".to_string(),
        ));
    };
    let snapshot = crate::blueprint::blueprint_version_snapshot(
        blueprint,
        &package.text_core.blueprint_characters,
        &package.text_core.blueprint_relationships,
    );
    let current = package
        .text_core
        .blueprint_versions
        .iter()
        .find(|version| {
            version.project_id == blueprint.project_id
                && version.version == blueprint.current_version
        })
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!(
                "项目族导入包缺少当前蓝图版本: project_id={}, version={}",
                blueprint.project_id, blueprint.current_version
            ))
        })?;
    if current.blueprint_data != snapshot {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "项目族导入包当前蓝图与版本快照不一致: blueprint_version_id={}",
            current.id
        )));
    }
    serde_json::to_string_pretty(&snapshot)
        .map(Some)
        .map_err(|error| {
            ProjectFamilyImportError::invalid_package(format!("序列化项目族蓝图快照失败: {error}"))
        })
}
