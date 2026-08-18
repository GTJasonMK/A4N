use crate::project_family_import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, map_optional, map_required,
    normalize_profile_evidence_role, remap_profile_id, remap_text_anchor_source_id,
};
use a4n_core::models::project::{profile_evidence, text_anchor};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};

pub(in crate::project_family_import::import_rows) async fn insert_text_anchor_rows(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let root_project_id = map_required(
        &maps.project_ids,
        package.root_project_id,
        "文本锚点 project_id",
    )?;
    insert_mapped_import_rows!(
        txn,
        &package.text_anchors,
        anchor,
        text_anchor::ActiveModel,
        maps.text_anchor_ids,
        "导入文本锚点失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.source_chapter_id = Set(map_optional(
                &maps.chapter_ids,
                anchor.source_chapter_id,
                "文本锚点 source_chapter_id",
            )?);
            active.source_snapshot_id = Set(map_optional(
                &maps.source_snapshot_ids,
                anchor.source_snapshot_id,
                "文本锚点 source_snapshot_id",
            )?);
            active.source_snapshot_chapter_id = Set(map_optional(
                &maps.source_snapshot_chapter_ids,
                anchor.source_snapshot_chapter_id,
                "文本锚点 source_snapshot_chapter_id",
            )?);
            active.source_id = Set(remap_text_anchor_source_id(anchor, maps)?);
        }
    );
    insert_import_rows!(
        txn,
        &package.profile_evidence,
        evidence,
        profile_evidence::ActiveModel,
        "导入档案证据失败",
        |active| {
            active.profile_id = Set(remap_profile_id(
                &evidence.profile_kind,
                evidence.profile_id,
                maps,
                "档案证据 profile_id",
            )?);
            active.text_anchor_id = Set(map_required(
                &maps.text_anchor_ids,
                evidence.text_anchor_id,
                "档案证据 text_anchor_id",
            )?);
            active.evidence_role =
                Set(normalize_profile_evidence_role(&evidence.evidence_role)?.to_string());
        }
    );
    Ok(())
}
