mod chapter;

use crate::text_anchor::SOURCE_KIND_CHAPTER_CONTENT;
use a4n_db_models::models::project::text_anchor;
use std::collections::{HashMap, HashSet};

use super::super::super::{ProjectFamilyPackage, hash_text};
use chapter::{validate_chapter_anchor_source, validate_chapter_content_anchor};

pub(in super::super) fn validate_text_anchor_manifest(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let chapters = package
        .text_core
        .chapters
        .iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    let source_snapshots = package
        .source_snapshots
        .iter()
        .map(|snapshot| snapshot.id)
        .collect::<HashSet<_>>();
    let snapshot_chapters = package
        .source_snapshot_chapters
        .iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    let part_outline_ids = package
        .text_core
        .part_outlines
        .iter()
        .map(|outline| outline.id)
        .collect::<HashSet<_>>();
    let chapter_outline_ids = package
        .text_core
        .chapter_outlines
        .iter()
        .map(|outline| outline.id)
        .collect::<HashSet<_>>();
    let blueprint_ids = package
        .text_core
        .blueprint
        .as_ref()
        .map(|blueprint| blueprint.id)
        .into_iter()
        .collect::<HashSet<_>>();
    let blueprint_version_ids = package
        .text_core
        .blueprint_versions
        .iter()
        .map(|version| version.id)
        .collect::<HashSet<_>>();

    for anchor in &package.text_anchors {
        if anchor.project_id != package.root_project_id {
            return Err(format!(
                "项目族导入包文本锚点不属于根项目: text_anchor_id={}, project_id={}",
                anchor.id, anchor.project_id
            ));
        }
        validate_text_anchor_offsets(anchor)?;
        let expected_quote_hash = hash_text(&anchor.quote);
        if anchor.quote_hash != expected_quote_hash {
            return Err(format!(
                "项目族导入包文本锚点 quote_hash 与 quote 不一致: text_anchor_id={}",
                anchor.id
            ));
        }
        match anchor.source_kind.as_str() {
            SOURCE_KIND_CHAPTER_CONTENT => validate_chapter_content_anchor(
                anchor,
                &chapters,
                &source_snapshots,
                &snapshot_chapters,
            )?,
            "chapter_summary" => validate_chapter_anchor_source(
                anchor,
                &chapters,
                &source_snapshots,
                &snapshot_chapters,
            )?,
            "outline" => validate_ambiguous_anchor_source(
                anchor.id,
                anchor.source_id,
                &part_outline_ids,
                &chapter_outline_ids,
                "outline",
            )?,
            "blueprint" => validate_ambiguous_anchor_source(
                anchor.id,
                anchor.source_id,
                &blueprint_ids,
                &blueprint_version_ids,
                "blueprint",
            )?,
            other => {
                return Err(format!(
                    "项目族导入包文本锚点 source_kind 无效: text_anchor_id={}, source_kind={other}",
                    anchor.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_text_anchor_offsets(anchor: &text_anchor::Model) -> Result<(), String> {
    if anchor.paragraph_index < 0
        || anchor.start_offset < 0
        || anchor.end_offset <= anchor.start_offset
    {
        return Err(format!(
            "项目族导入包文本锚点范围无效: text_anchor_id={}, paragraph_index={}, start_offset={}, end_offset={}",
            anchor.id, anchor.paragraph_index, anchor.start_offset, anchor.end_offset
        ));
    }
    Ok(())
}

fn validate_ambiguous_anchor_source(
    anchor_id: i64,
    source_id: i64,
    first_source_ids: &HashSet<i64>,
    second_source_ids: &HashSet<i64>,
    label: &str,
) -> Result<(), String> {
    match (
        first_source_ids.contains(&source_id),
        second_source_ids.contains(&source_id),
    ) {
        (true, false) | (false, true) => Ok(()),
        (true, true) => Err(format!(
            "项目族导入包文本锚点 {label} source_id 存在歧义: text_anchor_id={}, source_id={source_id}",
            anchor_id
        )),
        (false, false) => Err(format!(
            "项目族导入包文本锚点 {label} source_id 不存在: text_anchor_id={}, source_id={source_id}",
            anchor_id
        )),
    }
}
