use a4n_db_models::models::{
    canvas::{canvas_text_asset, canvas_text_asset_version},
    chapter::chapter,
    project::{
        novel_project, project_source_snapshot, source_snapshot_chapter,
        standalone_comic_text_source,
    },
};
use a4n_domain::{project::ProjectSourceMode, source_text_hash};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrepareStandaloneComicSourceDbInput {
    pub project_id: i64,
    pub asset_id: i64,
    pub version: i32,
    pub start_section: i32,
    pub end_section: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum StandaloneComicSourceDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    InvalidState(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),
}

struct TextSection {
    title: String,
    start: usize,
}

pub async fn prepare_standalone_comic_source_chapter(
    db: &DatabaseConnection,
    input: PrepareStandaloneComicSourceDbInput,
) -> Result<a4n_domain::chapter::ChapterRecord, StandaloneComicSourceDbError> {
    for (name, value) in [
        ("project_id", input.project_id),
        ("asset_id", input.asset_id),
    ] {
        if value <= 0 {
            return Err(StandaloneComicSourceDbError::InvalidArgument(format!(
                "{name} 必须是正整数"
            )));
        }
    }
    if input.version <= 0 || input.start_section < 0 || input.end_section < input.start_section {
        return Err(StandaloneComicSourceDbError::InvalidArgument(
            "素材版本和标题范围无效".to_string(),
        ));
    }

    let txn = db.begin().await?;
    let project = novel_project::Entity::find_by_id(input.project_id)
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or_else(|| {
            StandaloneComicSourceDbError::NotFound(format!("项目不存在: {}", input.project_id))
        })?;
    let source_mode = ProjectSourceMode::from_identity(
        &project.project_kind,
        project.root_project_id,
        project.source_snapshot_id,
    )
    .map_err(|error| {
        StandaloneComicSourceDbError::InvalidArgument(format!("项目 {}: {error}", project.id))
    })?;
    if source_mode != ProjectSourceMode::Standalone {
        return Err(StandaloneComicSourceDbError::InvalidArgument(
            "文本素材来源只适用于独立漫画项目".to_string(),
        ));
    }

    let asset = canvas_text_asset::Entity::find_by_id(input.asset_id)
        .one(&txn)
        .await?
        .ok_or_else(|| {
            StandaloneComicSourceDbError::NotFound(format!("文本素材不存在: {}", input.asset_id))
        })?;
    if asset.project_id != input.project_id {
        return Err(StandaloneComicSourceDbError::Forbidden(
            "文本素材不属于当前漫画项目".to_string(),
        ));
    }
    let version = canvas_text_asset_version::Entity::find()
        .filter(canvas_text_asset_version::Column::CanvasTextAssetId.eq(input.asset_id))
        .filter(canvas_text_asset_version::Column::Version.eq(input.version))
        .one(&txn)
        .await?
        .ok_or_else(|| {
            StandaloneComicSourceDbError::NotFound(format!(
                "文本素材版本不存在: {}/{}",
                input.asset_id, input.version
            ))
        })?;
    let (content, source_title) =
        select_text_sections(&version.content, input.start_section, input.end_section)?;
    let content_hash = source_text_hash(&content);

    if let Some(source) = standalone_comic_text_source::Entity::find()
        .filter(standalone_comic_text_source::Column::ProjectId.eq(input.project_id))
        .filter(standalone_comic_text_source::Column::CanvasTextAssetId.eq(input.asset_id))
        .filter(standalone_comic_text_source::Column::TextAssetVersion.eq(input.version))
        .filter(standalone_comic_text_source::Column::StartSectionIndex.eq(input.start_section))
        .filter(standalone_comic_text_source::Column::EndSectionIndex.eq(input.end_section))
        .one(&txn)
        .await?
    {
        let existing = chapter::Entity::find_by_id(source.chapter_id)
            .one(&txn)
            .await?
            .ok_or_else(|| {
                StandaloneComicSourceDbError::InvalidArgument(format!(
                    "漫画来源快照缺少章节: {}",
                    source.chapter_id
                ))
            })?;
        if existing.content != content
            || existing.title.as_deref() != Some(source_title.as_str())
            || source.content_hash != content_hash
        {
            return Err(StandaloneComicSourceDbError::InvalidState(
                "漫画来源快照与文本素材版本不一致".to_string(),
            ));
        }
        txn.commit().await?;
        return Ok(crate::project::chapter_record(existing));
    }

    let last_number = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::ChapterNumber)
        .filter(chapter::Column::ProjectId.eq(input.project_id))
        .order_by_desc(chapter::Column::ChapterNumber)
        .into_tuple::<i32>()
        .one(&txn)
        .await?
        .unwrap_or(0);
    let chapter_number = last_number.checked_add(1).ok_or_else(|| {
        StandaloneComicSourceDbError::InvalidArgument("漫画来源编号溢出".to_string())
    })?;
    let word_count = i32::try_from(content.chars().count()).map_err(|_| {
        StandaloneComicSourceDbError::InvalidArgument("文本素材长度超出支持范围".to_string())
    })?;
    let now = Utc::now().naive_utc();
    let chapter_row = chapter::ActiveModel {
        project_id: Set(input.project_id),
        outline_id: Set(None),
        chapter_number: Set(chapter_number),
        title: Set(Some(source_title.clone())),
        content: Set(content),
        word_count: Set(word_count),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(Some(json!({
            "source_type": "standalone_comic_text",
            "text_asset_id": input.asset_id,
            "text_asset_version": input.version,
            "start_section": input.start_section,
            "end_section": input.end_section,
            "content_sha256": content_hash.clone(),
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(input.project_id),
        source_version_hash: Set(content_hash.clone()),
        blueprint_version_id: Set(None),
        chapter_scope_json: Set(json!({
            "kind": "standalone_comic_text",
            "chapter_number": chapter_number,
        })),
        chapter_hashes_json: Set(json!([content_hash.clone()])),
        profile_hashes_json: Set(json!({})),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let snapshot_chapter = source_snapshot_chapter::ActiveModel {
        snapshot_id: Set(snapshot.id),
        root_project_id: Set(input.project_id),
        source_chapter_id: Set(chapter_row.id),
        chapter_number: Set(chapter_number),
        title: Set(chapter_row.title.clone()),
        content: Set(chapter_row.content.clone()),
        summary: Set(None),
        analysis_data_json: Set(chapter_row.analysis_data.clone()),
        content_hash: Set(content_hash.clone()),
        analysis_hash: Set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    standalone_comic_text_source::ActiveModel {
        project_id: Set(input.project_id),
        chapter_id: Set(chapter_row.id),
        source_snapshot_id: Set(snapshot.id),
        source_snapshot_chapter_id: Set(snapshot_chapter.id),
        canvas_text_asset_id: Set(input.asset_id),
        text_asset_version: Set(input.version),
        start_section_index: Set(input.start_section),
        end_section_index: Set(input.end_section),
        source_title: Set(source_title),
        content_hash: Set(content_hash),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    txn.commit().await?;
    Ok(crate::project::chapter_record(chapter_row))
}

pub(crate) fn select_text_sections(
    content: &str,
    start_section: i32,
    end_section: i32,
) -> Result<(String, String), StandaloneComicSourceDbError> {
    let sections = text_sections(content);
    let start = usize::try_from(start_section).map_err(|_| {
        StandaloneComicSourceDbError::InvalidArgument("起始标题范围无效".to_string())
    })?;
    let end = usize::try_from(end_section).map_err(|_| {
        StandaloneComicSourceDbError::InvalidArgument("结束标题范围无效".to_string())
    })?;
    if start > end || end >= sections.len() {
        return Err(StandaloneComicSourceDbError::InvalidArgument(format!(
            "标题范围超出素材范围: {start_section}..={end_section}，共 {} 段",
            sections.len()
        )));
    }
    let begin_offset = sections[start].start;
    let end_offset = sections
        .get(end + 1)
        .map_or(content.len(), |section| section.start);
    let source_title = if start == end {
        sections[start].title.clone()
    } else {
        format!("{} - {}", sections[start].title, sections[end].title)
    };
    Ok((
        content[begin_offset..end_offset].trim().to_string(),
        source_title,
    ))
}

fn text_sections(content: &str) -> Vec<TextSection> {
    let mut sections = Vec::new();
    let mut offset = 0;
    for line in content.split_inclusive('\n') {
        let heading = line.trim_end_matches(['\r', '\n']);
        if let Some(title) = section_title(heading) {
            sections.push(TextSection {
                title,
                start: offset,
            });
        }
        offset += line.len();
    }
    if sections.is_empty() {
        sections.push(TextSection {
            title: "全文".to_string(),
            start: 0,
        });
    }
    sections
}

fn section_title(line: &str) -> Option<String> {
    let heading_marks = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if (1..=6).contains(&heading_marks)
        && line
            .as_bytes()
            .get(heading_marks)
            .is_some_and(u8::is_ascii_whitespace)
    {
        let title = line[heading_marks..].trim_start();
        if !title.is_empty() {
            return Some(title.to_string());
        }
    }
    if !line.starts_with('第') {
        return None;
    }
    let marker = line
        .char_indices()
        .skip(2)
        .take(32)
        .find(|(_, character)| matches!(character, '章' | '节' | '幕' | '集'));
    marker.map(|_| line.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_contiguous_heading_range_from_frozen_text() {
        let source = "前言\n\n第一章 起点\n正文一\n第二章 转折\n正文二\n第三章 结尾\n正文三";
        assert_eq!(
            select_text_sections(source, 0, 1).unwrap(),
            (
                "第一章 起点\n正文一\n第二章 转折\n正文二".to_string(),
                "第一章 起点 - 第二章 转折".to_string()
            )
        );
    }

    #[test]
    fn rejects_reversed_or_out_of_range_title_selection() {
        assert!(select_text_sections("# A\n# B", 1, 0).is_err());
        assert!(select_text_sections("# A", 0, 1).is_err());
    }
}
