use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::{blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint},
};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, NotSet, Set};

use crate::project_family_import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, map_optional,
};

pub(super) async fn insert_story_rows(
    txn: &DatabaseTransaction,
    root_project_id: i64,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
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
    Ok(())
}
