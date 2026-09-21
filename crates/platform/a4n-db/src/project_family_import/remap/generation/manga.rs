use a4n_domain::manga::{
    MangaPageGenerationLayoutV2, MangaPagePromptSourceSummary, MangaReferenceAssetKind,
    MangaReferenceBindingSummary,
};
use serde_json::Value;

use crate::project_family_import::{
    ImportIdMaps, ProjectFamilyImportError, map_optional, map_required,
};

pub(in crate::project_family_import) fn remap_manga_page_layout(
    value: &Value,
    maps: &ImportIdMaps,
    expected_page_number: u32,
    label: &str,
) -> Result<Value, ProjectFamilyImportError> {
    let mut layout: MangaPageGenerationLayoutV2 =
        serde_json::from_value(value.clone()).map_err(|error| {
            ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
        })?;
    layout.validate(expected_page_number).map_err(|error| {
        ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
    })?;

    remap_manga_page_source(&mut layout.source, maps, label)?;
    remap_manga_reference_bindings(&mut layout.reference_bindings, maps, label)?;
    serde_json::to_value(layout).map_err(|error| {
        ProjectFamilyImportError::invalid_package(format!("{label} 序列化失败: {error}"))
    })
}

pub(in crate::project_family_import) fn remap_manga_reference_bindings_value(
    value: &Value,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<Value, ProjectFamilyImportError> {
    let mut bindings: Vec<MangaReferenceBindingSummary> = serde_json::from_value(value.clone())
        .map_err(|error| {
            ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
        })?;
    remap_manga_reference_bindings(&mut bindings, maps, label)?;
    serde_json::to_value(bindings).map_err(|error| {
        ProjectFamilyImportError::invalid_package(format!("{label} 序列化失败: {error}"))
    })
}

fn remap_manga_page_source(
    source: &mut MangaPagePromptSourceSummary,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let old_chapter_id = source.source_chapter_id;
    let old_snapshot_id = source.source_snapshot_id;
    let old_snapshot_chapter_id = source.source_snapshot_chapter_id;
    let expected_revision_key = match (old_snapshot_id, old_snapshot_chapter_id) {
        (None, None) => format!("live:{old_chapter_id}:{}", source.content_sha256),
        (Some(snapshot_id), Some(snapshot_chapter_id)) => format!(
            "snapshot:{snapshot_id}:chapter:{snapshot_chapter_id}:{}",
            source.content_sha256
        ),
        _ => {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "{label}.source 的 source_snapshot_id 与 source_snapshot_chapter_id 必须同时存在或同时为空"
            )));
        }
    };
    if source.source_revision_key != expected_revision_key {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "{label}.source.source_revision_key 与来源身份不一致"
        )));
    }

    source.root_project_id = map_required(
        &maps.project_ids,
        source.root_project_id,
        &format!("{label}.source.root_project_id"),
    )?;
    source.source_chapter_id = map_required(
        &maps.chapter_ids,
        old_chapter_id,
        &format!("{label}.source.source_chapter_id"),
    )?;
    source.source_snapshot_id = map_optional(
        &maps.source_snapshot_ids,
        old_snapshot_id,
        &format!("{label}.source.source_snapshot_id"),
    )?;
    source.source_snapshot_chapter_id = map_optional(
        &maps.source_snapshot_chapter_ids,
        old_snapshot_chapter_id,
        &format!("{label}.source.source_snapshot_chapter_id"),
    )?;
    let old_shooting_script_id = source.shooting_script_id;
    source.shooting_script_id = map_required(
        &maps.shooting_script_ids,
        old_shooting_script_id,
        &format!("{label}.source.shooting_script_id"),
    )?;
    source.shooting_script_sha256 = maps
        .shooting_script_sha256s
        .get(&old_shooting_script_id)
        .cloned()
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!(
                "{label}.source.shooting_script_sha256 引用了导入包外或尚未导入的拍摄剧本: {old_shooting_script_id}"
            ))
        })?;
    source.source_revision_key =
        match (source.source_snapshot_id, source.source_snapshot_chapter_id) {
            (None, None) => format!(
                "live:{}:{}",
                source.source_chapter_id, source.content_sha256
            ),
            (Some(snapshot_id), Some(snapshot_chapter_id)) => format!(
                "snapshot:{snapshot_id}:chapter:{snapshot_chapter_id}:{}",
                source.content_sha256
            ),
            _ => unreachable!("source snapshot identity was validated before remapping"),
        };
    Ok(())
}

fn remap_manga_reference_bindings(
    bindings: &mut [MangaReferenceBindingSummary],
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    for (index, binding) in bindings.iter_mut().enumerate() {
        if binding.character_name.trim().is_empty() {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "{label}.reference_bindings[{index}].character_name 不能为空"
            )));
        }
        let Some(asset) = binding.bound_asset.as_mut() else {
            continue;
        };
        let (ids, paths, kind) = match asset.asset_kind {
            MangaReferenceAssetKind::CharacterPortrait => (
                &maps.character_portrait_ids,
                &maps.character_portrait_paths,
                "character_portrait",
            ),
            MangaReferenceAssetKind::GeneratedImage => (
                &maps.generated_image_ids,
                &maps.generated_image_paths,
                "generated_image",
            ),
        };
        let old_id = asset.asset_id;
        asset.asset_id = map_required(
            ids,
            old_id,
            &format!("{label}.reference_bindings[{index}].bound_asset.asset_id"),
        )?;
        asset.image_path = paths.get(&old_id).cloned().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!(
                "{label}.reference_bindings[{index}] 引用了导入包外或尚未导入的 {kind} 文件: {old_id}"
            ))
        })?;
    }
    Ok(())
}
