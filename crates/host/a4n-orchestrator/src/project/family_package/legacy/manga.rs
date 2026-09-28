use std::collections::{BTreeMap, HashMap};

use a4n_domain::{
    manga::{MangaPagePlanSchemaVersion, MangaPagePlanV4, MangaPagePromptSourceSummary},
    media_authoring::{
        AdaptationPlanV1, AdaptationPlanVersion, AdaptationTarget, AuthoringDocumentRef,
        legacy::{historical_manga_page, saved_manga_source, upgrade_saved_manga_layout},
    },
};
use serde_json::json;

use super::{
    ProjectFamilyPackage, error, insert_document, next_id, receipt,
    source::{historical_project, source_for},
};

pub(super) fn upgrade(
    package: &mut ProjectFamilyPackage,
    media_bytes: &mut HashMap<String, Vec<u8>>,
    next: &mut i64,
) -> Result<(), String> {
    let mut groups = BTreeMap::new();
    let mut originals = std::mem::take(&mut package.chapter_manga_prompts);
    originals.sort_by_key(|row| (row.page_number, row.id));
    for row in originals {
        let layout = row.layout.as_ref().ok_or("旧漫画缺少已保存布局")?;
        let old = saved_manga_source(layout)?;
        if old.root_project_id != package.root_project_id || old.source_chapter_id != row.chapter_id
        {
            return Err("旧漫画原作或章节身份不一致".into());
        }
        let source = source_for(
            package,
            row.chapter_id,
            old.source_snapshot_id,
            old.source_snapshot_chapter_id,
            &old.content_sha256,
            next,
        )?;
        let owner = package
            .projects
            .iter()
            .find(|project| project.id == row.project_id)
            .ok_or("旧漫画所属项目不存在")?;
        let (project_id, current) = match owner.project_kind.as_str() {
            "comic" if owner.root_project_id == Some(package.root_project_id) => (
                owner.id,
                owner.source_snapshot_id == Some(source.snapshot_id),
            ),
            "text" if owner.id == package.root_project_id => (
                historical_project(package, source.snapshot_id, "comic", next)?,
                true,
            ),
            _ => return Err("旧漫画项目类型或原作不一致".into()),
        };
        let (unit, page) = historical_manga_page(row.id, layout)?;
        groups
            .entry((project_id, source.snapshot_id, row.chapter_id))
            .or_insert_with(|| (source, current, Vec::new()))
            .2
            .push((row, unit, page));
    }
    for ((project_id, _, _), (source, current, rows)) in groups {
        let plan = AdaptationPlanV1 {
            schema_version: AdaptationPlanVersion::V1,
            target: AdaptationTarget::Manga,
            intention: "保留已保存的页面、画格、对白和人工编辑；原始导演引用保留在迁移回执。"
                .into(),
            decisions: Vec::new(),
            units: rows.iter().map(|(_, unit, _)| unit.clone()).collect(),
        };
        plan.validate(&Default::default())?;
        let doc = insert_document(
            package,
            project_id,
            &source,
            "adaptation_plan",
            serde_json::to_value(plan).map_err(error)?,
            Vec::new(),
            current,
            next,
        )?;
        let pages = rows
            .iter()
            .map(|(_, _, page)| page.clone())
            .collect::<Vec<_>>();
        let page_plan_ref = if pages
            .iter()
            .enumerate()
            .all(|(index, page)| usize::try_from(page.page_number).ok() == Some(index + 1))
        {
            let page_plan = MangaPagePlanV4 {
                schema_version: MangaPagePlanSchemaVersion::V4,
                adaptation_plan_id: doc.id,
                adaptation_plan_revision: doc.revision,
                pages,
            };
            page_plan.validate().map_err(error)?;
            let page_doc = insert_document(
                package,
                project_id,
                &source,
                "manga_page_plan",
                serde_json::to_value(page_plan).map_err(error)?,
                vec![AuthoringDocumentRef {
                    id: doc.id,
                    revision: doc.revision,
                    sha256: doc.document_sha256.clone(),
                }],
                current,
                next,
            )?;
            Some(AuthoringDocumentRef {
                id: page_doc.id,
                revision: page_doc.revision,
                sha256: page_doc.document_sha256,
            })
        } else {
            let value =
                json!({"reason": "保存页面不连续，不能标记完整分页计划已采用", "pages": pages});
            receipt(
                package,
                "partial_manga_page_plan",
                doc.id,
                doc.id,
                project_id,
                value.clone(),
                &value,
            )?;
            None
        };
        for (mut row, _, page) in rows {
            let before = serde_json::to_value(&row).map_err(error)?;
            if row.project_id != project_id {
                let images = package
                    .generated_images
                    .iter()
                    .filter(|image| {
                        image.project_id == row.project_id
                            && image.image_type == "manga_page"
                            && (Some(image.id) == row.generated_image_id
                                || image
                                    .metadata_json
                                    .as_ref()
                                    .and_then(|value| value.get("page_prompt_id"))
                                    .and_then(serde_json::Value::as_i64)
                                    == Some(row.id))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let mut adopted = None;
                for image in images {
                    if image.chapter_id != Some(row.chapter_id) {
                        return Err("旧漫画图片章节不一致".into());
                    }
                    let mut copy = image.clone();
                    copy.id = next_id(next)?;
                    copy.project_id = project_id;
                    copy_image_entry(package, media_bytes, image.id, copy.id)?;
                    if Some(image.id) == row.generated_image_id {
                        adopted = Some(copy.id);
                    }
                    receipt(
                        package,
                        "manga_image_copy",
                        image.id,
                        copy.id,
                        project_id,
                        serde_json::to_value(image).map_err(error)?,
                        &copy,
                    )?;
                    package.generated_images.push(copy);
                }
                if row.generated_image_id.is_some() && adopted.is_none() {
                    return Err("旧漫画已采用图片不存在".into());
                }
                row.generated_image_id = adopted;
            }
            row.project_id = project_id;
            row.layout = Some(upgrade_saved_manga_layout(
                row.layout.as_ref().ok_or("旧漫画布局丢失")?,
                MangaPagePromptSourceSummary {
                    root_project_id: source.root_project_id,
                    source_chapter_id: source.source_chapter_id,
                    source_snapshot_id: Some(source.snapshot_id),
                    source_snapshot_chapter_id: Some(source.id),
                    source_revision_key: format!(
                        "snapshot:{}:chapter:{}:{}",
                        source.snapshot_id, source.id, source.content_hash
                    ),
                    content_sha256: source.content_hash.clone(),
                    adaptation_plan_id: doc.id,
                    adaptation_plan_revision: doc.revision,
                    adaptation_plan_sha256: doc.document_sha256.clone(),
                },
                page.clone(),
            )?);
            row.layout.as_mut().ok_or("漫画布局丢失")?["page_plan_ref"] =
                serde_json::to_value(&page_plan_ref).map_err(error)?;
            receipt(
                package,
                "manga_layout",
                row.id,
                row.id,
                project_id,
                before,
                &row,
            )?;
            package.chapter_manga_prompts.push(row);
        }
    }
    Ok(())
}

fn copy_image_entry(
    package: &mut ProjectFamilyPackage,
    media_bytes: &mut HashMap<String, Vec<u8>>,
    old_id: i64,
    new_id: i64,
) -> Result<(), String> {
    let entries = package
        .media_files
        .iter()
        .filter(|entry| entry.owner_kind == "generated_image" && entry.owner_id == old_id)
        .collect::<Vec<_>>();
    if entries.len() != 1 || entries[0].field != "file_path" {
        return Err("旧漫画图片媒体清单缺失或重复".into());
    }
    let mut entry = (*entries[0]).clone();
    let old_path = super::super::family_media_package_path(
        &entry.owner_kind,
        old_id,
        &entry.field,
        &entry.original_path,
    )?;
    if entry.package_path != old_path {
        return Err("旧漫画媒体归档路径无效".into());
    }
    let bytes = media_bytes
        .get(&entry.package_path)
        .filter(|bytes| !bytes.is_empty())
        .ok_or("旧漫画图片字节缺失")?
        .clone();
    entry.owner_id = new_id;
    entry.package_path = super::super::family_media_package_path(
        &entry.owner_kind,
        new_id,
        &entry.field,
        &entry.original_path,
    )?;
    if media_bytes
        .insert(entry.package_path.clone(), bytes)
        .is_some()
    {
        return Err("迁移漫画媒体归档路径冲突".into());
    }
    package.media_files.push(entry);
    Ok(())
}
