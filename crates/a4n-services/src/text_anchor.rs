//! 文本锚点与档案证据服务。

use a4n_core::error::AppError;
use a4n_core::models::{
    asset::object_profile,
    chapter::chapter,
    character::protagonist_profile,
    project::{profile_evidence, project_source_snapshot, source_snapshot_chapter, text_anchor},
    scene::scene_profile,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use crate::chapter::source_spans::source_spans_for_content;
use crate::project::PROJECT_KIND_TEXT;

pub const SOURCE_KIND_CHAPTER_CONTENT: &str = "chapter_content";
pub const PROFILE_KIND_CHARACTER: &str = "character";
pub const PROFILE_KIND_OBJECT: &str = "object";
pub const PROFILE_KIND_SCENE: &str = "scene";

pub const EVIDENCE_ROLE_FIRST_SEEN: &str = "first_seen";
pub const EVIDENCE_ROLE_APPEARANCE: &str = "appearance";
pub const EVIDENCE_ROLE_STATE_CHANGE: &str = "state_change";
pub const EVIDENCE_ROLE_LOCATION: &str = "location";
pub const EVIDENCE_ROLE_RELATIONSHIP: &str = "relationship";
pub const EVIDENCE_ROLE_BEHAVIOR: &str = "behavior";

#[derive(Clone)]
pub struct TextAnchorService {
    db: Arc<DatabaseConnection>,
}

#[derive(Debug, Clone)]
pub struct ProfileEvidenceInput {
    pub text_anchor_id: i64,
    pub evidence_role: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NormalizedProfileKind {
    Character,
    Object,
    Scene,
}

impl NormalizedProfileKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Character => PROFILE_KIND_CHARACTER,
            Self::Object => PROFILE_KIND_OBJECT,
            Self::Scene => PROFILE_KIND_SCENE,
        }
    }
}

pub(crate) struct InsertContentAnchorsInput<'a> {
    pub project_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_id: i64,
    pub chapter_number: i32,
    pub content: &'a str,
    pub source_version_hash: &'a str,
}

pub fn text_anchor_prompt_values(
    chapters: &[chapter::Model],
    anchors: &[text_anchor::Model],
) -> Vec<Value> {
    let chapter_numbers = chapters
        .iter()
        .map(|chapter| (chapter.id, chapter.chapter_number))
        .collect::<HashMap<_, _>>();
    anchors
        .iter()
        .map(|anchor| {
            let chapter_number = anchor
                .source_chapter_id
                .and_then(|chapter_id| chapter_numbers.get(&chapter_id).copied());
            json!({
                "id": anchor.id,
                "chapter_label": chapter_number.map(|value| format!("第{value}章")),
                "quote": anchor.quote,
            })
        })
        .collect()
}

impl TextAnchorService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn rebuild_chapter_content_anchors(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<text_anchor::Model>, AppError> {
        let project = self.ensure_text_root_project(project_id).await?;
        let chapter = chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
        if chapter.project_id != project.id {
            return Err(AppError::InvalidArgument(format!(
                "章节不属于文本根项目: chapter_id={chapter_id}, project_id={project_id}"
            )));
        }

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        text_anchor::Entity::delete_many()
            .filter(text_anchor::Column::ProjectId.eq(project.id))
            .filter(text_anchor::Column::SourceKind.eq(SOURCE_KIND_CHAPTER_CONTENT))
            .filter(text_anchor::Column::SourceId.eq(chapter.id))
            .filter(text_anchor::Column::SourceSnapshotId.is_null())
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;

        let source_version_hash = hash_text(&chapter.content);
        let anchors = insert_content_anchors(
            &tx,
            InsertContentAnchorsInput {
                project_id: project.id,
                source_chapter_id: Some(chapter.id),
                source_snapshot_id: None,
                source_snapshot_chapter_id: None,
                source_id: chapter.id,
                chapter_number: chapter.chapter_number,
                content: &chapter.content,
                source_version_hash: &source_version_hash,
            },
        )
        .await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(anchors)
    }

    pub async fn ensure_chapter_content_anchors(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<text_anchor::Model>, AppError> {
        let project = self.ensure_text_root_project(project_id).await?;
        let chapter = chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
        if chapter.project_id != project.id {
            return Err(AppError::InvalidArgument(format!(
                "章节不属于文本根项目: chapter_id={chapter_id}, project_id={project_id}"
            )));
        }
        let source_version_hash = hash_text(&chapter.content);
        let existing = text_anchor::Entity::find()
            .filter(text_anchor::Column::ProjectId.eq(project.id))
            .filter(text_anchor::Column::SourceKind.eq(SOURCE_KIND_CHAPTER_CONTENT))
            .filter(text_anchor::Column::SourceId.eq(chapter.id))
            .filter(text_anchor::Column::SourceSnapshotId.is_null())
            .order_by_asc(text_anchor::Column::StartOffset)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if !existing.is_empty()
            && existing
                .iter()
                .all(|anchor| anchor.source_version_hash == source_version_hash)
        {
            return Ok(existing);
        }
        self.rebuild_chapter_content_anchors(project.id, chapter.id)
            .await
    }

    pub async fn read_current_chapter_content_anchors(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<text_anchor::Model>, AppError> {
        let project = self.ensure_text_root_project(project_id).await?;
        let chapter = chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
        if chapter.project_id != project.id {
            return Err(AppError::InvalidArgument(format!(
                "章节不属于文本根项目: chapter_id={chapter_id}, project_id={project_id}"
            )));
        }
        let source_version_hash = hash_text(&chapter.content);
        let existing = text_anchor::Entity::find()
            .filter(text_anchor::Column::ProjectId.eq(project.id))
            .filter(text_anchor::Column::SourceKind.eq(SOURCE_KIND_CHAPTER_CONTENT))
            .filter(text_anchor::Column::SourceId.eq(chapter.id))
            .filter(text_anchor::Column::SourceSnapshotId.is_null())
            .order_by_asc(text_anchor::Column::StartOffset)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        validate_current_chapter_content_anchors(&chapter, existing, &source_version_hash)
    }

    pub async fn rebuild_snapshot_chapter_content_anchors(
        &self,
        root_project_id: i64,
        source_snapshot_chapter_id: i64,
    ) -> Result<Vec<text_anchor::Model>, AppError> {
        let project = self.ensure_text_root_project(root_project_id).await?;
        let snapshot_chapter =
            source_snapshot_chapter::Entity::find_by_id(source_snapshot_chapter_id)
                .one(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| {
                    AppError::NotFound(format!("源快照章节不存在: {source_snapshot_chapter_id}"))
                })?;
        if snapshot_chapter.root_project_id != project.id {
            return Err(AppError::InvalidArgument(format!(
                "源快照章节不属于文本根项目: source_snapshot_chapter_id={source_snapshot_chapter_id}, root_project_id={root_project_id}"
            )));
        }
        let snapshot = project_source_snapshot::Entity::find_by_id(snapshot_chapter.snapshot_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "源文本快照不存在: {}",
                    snapshot_chapter.snapshot_id
                ))
            })?;
        if snapshot.root_project_id != project.id {
            return Err(AppError::InvalidArgument(format!(
                "源文本快照不属于文本根项目: snapshot_id={}, root_project_id={root_project_id}",
                snapshot.id
            )));
        }

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        text_anchor::Entity::delete_many()
            .filter(text_anchor::Column::ProjectId.eq(project.id))
            .filter(text_anchor::Column::SourceSnapshotId.eq(snapshot.id))
            .filter(text_anchor::Column::SourceSnapshotChapterId.eq(snapshot_chapter.id))
            .filter(text_anchor::Column::SourceKind.eq(SOURCE_KIND_CHAPTER_CONTENT))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;

        let anchors = insert_content_anchors(
            &tx,
            InsertContentAnchorsInput {
                project_id: project.id,
                source_chapter_id: Some(snapshot_chapter.source_chapter_id),
                source_snapshot_id: Some(snapshot.id),
                source_snapshot_chapter_id: Some(snapshot_chapter.id),
                source_id: snapshot_chapter.id,
                chapter_number: snapshot_chapter.chapter_number,
                content: &snapshot_chapter.content,
                source_version_hash: &snapshot_chapter.content_hash,
            },
        )
        .await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(anchors)
    }

    pub async fn validate_anchor_refs(
        &self,
        root_project_id: i64,
        source_snapshot_id: Option<i64>,
        source_snapshot_chapter_ids: &[i64],
        anchor_ids: &[i64],
    ) -> Result<Vec<text_anchor::Model>, AppError> {
        self.ensure_text_root_project(root_project_id).await?;
        if let Some(snapshot_id) = source_snapshot_id {
            self.validate_snapshot_filters(root_project_id, Some(snapshot_id), None)
                .await?;
        }
        if anchor_ids.is_empty() {
            return Err(AppError::InvalidArgument(
                "text_anchor_ids 不能为空".to_string(),
            ));
        }
        let mut unique_ids = BTreeSet::new();
        for id in anchor_ids {
            if *id <= 0 {
                return Err(AppError::InvalidArgument(format!(
                    "text_anchor_id 必须为正整数: {id}"
                )));
            }
            unique_ids.insert(*id);
        }
        let anchors = text_anchor::Entity::find()
            .filter(text_anchor::Column::Id.is_in(unique_ids.iter().copied()))
            .order_by_asc(text_anchor::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if anchors.len() != unique_ids.len() {
            return Err(AppError::InvalidArgument(
                "存在不存在的 text_anchor_id".to_string(),
            ));
        }
        let allowed_chapters = source_snapshot_chapter_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        for anchor in &anchors {
            if anchor.project_id != root_project_id {
                return Err(AppError::InvalidArgument(format!(
                    "text_anchor_id={} 不属于文本根项目 {}",
                    anchor.id, root_project_id
                )));
            }
            if anchor.source_snapshot_id != source_snapshot_id {
                return Err(AppError::InvalidArgument(format!(
                    "text_anchor_id={} 不属于目标源快照",
                    anchor.id
                )));
            }
            if !allowed_chapters.is_empty() {
                let chapter_id = anchor.source_snapshot_chapter_id.ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "text_anchor_id={} 不是源快照章节锚点",
                        anchor.id
                    ))
                })?;
                if !allowed_chapters.contains(&chapter_id) {
                    return Err(AppError::InvalidArgument(format!(
                        "text_anchor_id={} 不属于目标章节范围",
                        anchor.id
                    )));
                }
            }
        }
        Ok(anchors)
    }

    pub async fn replace_profile_evidence(
        &self,
        root_project_id: i64,
        source_snapshot_id: Option<i64>,
        profile_kind: &str,
        profile_id: i64,
        refs: Vec<ProfileEvidenceInput>,
    ) -> Result<Vec<profile_evidence::Model>, AppError> {
        let profile_kind = normalize_profile_kind(profile_kind)?;
        let profile_kind_value = profile_kind.as_str();
        if refs.is_empty() {
            return Err(AppError::InvalidArgument(
                "profile_evidence 不能为空".to_string(),
            ));
        }
        self.ensure_profile_belongs_to_root(root_project_id, profile_kind, profile_id)
            .await?;
        let mut anchor_ids = Vec::with_capacity(refs.len());
        let mut seen = BTreeSet::new();
        for item in &refs {
            let role = normalize_evidence_role(&item.evidence_role)?;
            if !seen.insert((item.text_anchor_id, role.to_string())) {
                return Err(AppError::InvalidArgument(format!(
                    "profile_evidence 重复: text_anchor_id={}, evidence_role={role}",
                    item.text_anchor_id
                )));
            }
            anchor_ids.push(item.text_anchor_id);
        }
        self.validate_anchor_refs(root_project_id, source_snapshot_id, &[], &anchor_ids)
            .await?;

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        profile_evidence::Entity::delete_many()
            .filter(profile_evidence::Column::ProfileKind.eq(profile_kind_value))
            .filter(profile_evidence::Column::ProfileId.eq(profile_id))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;
        let now = Utc::now().naive_utc();
        let mut inserted = Vec::with_capacity(refs.len());
        for item in refs {
            inserted.push(
                profile_evidence::ActiveModel {
                    profile_kind: Set(profile_kind_value.to_string()),
                    profile_id: Set(profile_id),
                    text_anchor_id: Set(item.text_anchor_id),
                    evidence_role: Set(normalize_evidence_role(&item.evidence_role)?.to_string()),
                    created_at: Set(now),
                    ..Default::default()
                }
                .insert(&tx)
                .await
                .map_err(AppError::Database)?,
            );
        }
        tx.commit().await.map_err(AppError::Database)?;
        Ok(inserted)
    }

    pub async fn merge_profile_evidence(
        &self,
        root_project_id: i64,
        source_snapshot_id: Option<i64>,
        profile_kind: &str,
        profile_id: i64,
        refs: Vec<ProfileEvidenceInput>,
    ) -> Result<Vec<profile_evidence::Model>, AppError> {
        let profile_kind = normalize_profile_kind(profile_kind)?;
        let profile_kind_value = profile_kind.as_str();
        if refs.is_empty() {
            return Err(AppError::InvalidArgument(
                "profile_evidence 不能为空".to_string(),
            ));
        }
        self.ensure_profile_belongs_to_root(root_project_id, profile_kind, profile_id)
            .await?;
        let mut anchor_ids = Vec::with_capacity(refs.len());
        let mut seen = BTreeSet::new();
        let mut normalized_refs = Vec::with_capacity(refs.len());
        for item in refs {
            let role = normalize_evidence_role(&item.evidence_role)?;
            if seen.insert((item.text_anchor_id, role.to_string())) {
                anchor_ids.push(item.text_anchor_id);
                normalized_refs.push(ProfileEvidenceInput {
                    text_anchor_id: item.text_anchor_id,
                    evidence_role: role.to_string(),
                });
            }
        }
        self.validate_anchor_refs(root_project_id, source_snapshot_id, &[], &anchor_ids)
            .await?;

        let existing = profile_evidence::Entity::find()
            .filter(profile_evidence::Column::ProfileKind.eq(profile_kind_value))
            .filter(profile_evidence::Column::ProfileId.eq(profile_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .map(|item| (item.text_anchor_id, item.evidence_role))
            .collect::<BTreeSet<_>>();
        let now = Utc::now().naive_utc();
        let mut inserted = Vec::new();
        for item in normalized_refs {
            if existing.contains(&(item.text_anchor_id, item.evidence_role.clone())) {
                continue;
            }
            inserted.push(
                profile_evidence::ActiveModel {
                    profile_kind: Set(profile_kind_value.to_string()),
                    profile_id: Set(profile_id),
                    text_anchor_id: Set(item.text_anchor_id),
                    evidence_role: Set(item.evidence_role),
                    created_at: Set(now),
                    ..Default::default()
                }
                .insert(self.db.as_ref())
                .await
                .map_err(AppError::Database)?,
            );
        }
        Ok(inserted)
    }

    async fn ensure_text_root_project(
        &self,
        project_id: i64,
    ) -> Result<a4n_core::models::project::novel_project::Model, AppError> {
        let project = a4n_core::models::project::novel_project::Entity::find_by_id(project_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("文本根项目不存在: {project_id}")))?;
        if project.project_kind != PROJECT_KIND_TEXT {
            return Err(AppError::InvalidArgument(format!(
                "项目不是文本根项目: {project_id}"
            )));
        }
        Ok(project)
    }

    async fn validate_snapshot_filters(
        &self,
        root_project_id: i64,
        source_snapshot_id: Option<i64>,
        source_snapshot_chapter_id: Option<i64>,
    ) -> Result<(), AppError> {
        match (source_snapshot_id, source_snapshot_chapter_id) {
            (None, Some(_)) => Err(AppError::InvalidArgument(
                "source_snapshot_chapter_id 需要同时提供 source_snapshot_id".to_string(),
            )),
            (Some(snapshot_id), chapter_id) => {
                let snapshot = project_source_snapshot::Entity::find_by_id(snapshot_id)
                    .one(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| {
                        AppError::NotFound(format!("源文本快照不存在: {snapshot_id}"))
                    })?;
                if snapshot.root_project_id != root_project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "源文本快照不属于文本根项目: snapshot_id={snapshot_id}, root_project_id={root_project_id}"
                    )));
                }
                if let Some(chapter_id) = chapter_id {
                    let chapter = source_snapshot_chapter::Entity::find_by_id(chapter_id)
                        .one(self.db.as_ref())
                        .await
                        .map_err(AppError::Database)?
                        .ok_or_else(|| {
                            AppError::NotFound(format!("源快照章节不存在: {chapter_id}"))
                        })?;
                    if chapter.snapshot_id != snapshot_id
                        || chapter.root_project_id != root_project_id
                    {
                        return Err(AppError::InvalidArgument(format!(
                            "源快照章节不属于目标源快照: source_snapshot_chapter_id={chapter_id}"
                        )));
                    }
                }
                Ok(())
            }
            (None, None) => Ok(()),
        }
    }

    async fn ensure_profile_belongs_to_root(
        &self,
        root_project_id: i64,
        profile_kind: NormalizedProfileKind,
        profile_id: i64,
    ) -> Result<(), AppError> {
        self.ensure_text_root_project(root_project_id).await?;
        match profile_kind {
            NormalizedProfileKind::Character => {
                let profile = protagonist_profile::Entity::find_by_id(profile_id)
                    .one(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("人物档案不存在: {profile_id}")))?;
                if profile.project_id != root_project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "人物档案不属于文本根项目: profile_id={profile_id}"
                    )));
                }
            }
            NormalizedProfileKind::Object => {
                let profile = object_profile::Entity::find_by_id(profile_id)
                    .one(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("物体档案不存在: {profile_id}")))?;
                if profile.project_id != root_project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "物体档案不属于文本根项目: profile_id={profile_id}"
                    )));
                }
            }
            NormalizedProfileKind::Scene => {
                let profile = scene_profile::Entity::find_by_id(profile_id)
                    .one(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("场景档案不存在: {profile_id}")))?;
                if profile.project_id != root_project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "场景档案不属于文本根项目: profile_id={profile_id}"
                    )));
                }
            }
        }
        Ok(())
    }
}

fn validate_current_chapter_content_anchors(
    chapter: &chapter::Model,
    anchors: Vec<text_anchor::Model>,
    source_version_hash: &str,
) -> Result<Vec<text_anchor::Model>, AppError> {
    if anchors.is_empty() {
        return Err(AppError::InvalidState {
            current: format!("第 {} 章缺少当前正文文本锚点", chapter.chapter_number),
            operation: "预览模型请求".to_string(),
        });
    }
    if anchors
        .iter()
        .any(|anchor| anchor.source_version_hash != source_version_hash)
    {
        return Err(AppError::InvalidState {
            current: format!("第 {} 章正文文本锚点已过期", chapter.chapter_number),
            operation: "预览模型请求".to_string(),
        });
    }
    Ok(anchors)
}

pub(crate) async fn insert_content_anchors<C>(
    db: &C,
    input: InsertContentAnchorsInput<'_>,
) -> Result<Vec<text_anchor::Model>, AppError>
where
    C: sea_orm::ConnectionTrait,
{
    let InsertContentAnchorsInput {
        project_id,
        source_chapter_id,
        source_snapshot_id,
        source_snapshot_chapter_id,
        source_id,
        chapter_number,
        content,
        source_version_hash,
    } = input;
    let now = Utc::now().naive_utc();
    let spans = source_spans_for_content(
        source_chapter_id.unwrap_or(source_id),
        chapter_number,
        content,
    );
    let mut anchors = Vec::with_capacity(spans.len());
    for span in spans {
        anchors.push(
            text_anchor::ActiveModel {
                project_id: Set(project_id),
                source_chapter_id: Set(source_chapter_id),
                source_snapshot_id: Set(source_snapshot_id),
                source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
                source_kind: Set(SOURCE_KIND_CHAPTER_CONTENT.to_string()),
                source_id: Set(source_id),
                paragraph_index: Set(to_i32(span.sentence_index, "paragraph_index")?),
                start_offset: Set(to_i32(span.char_start, "start_offset")?),
                end_offset: Set(to_i32(span.char_end, "end_offset")?),
                quote_hash: Set(hash_text(&span.text)),
                quote: Set(span.text),
                source_version_hash: Set(source_version_hash.to_string()),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(AppError::Database)?,
        );
    }
    Ok(anchors)
}

fn normalize_profile_kind(kind: &str) -> Result<NormalizedProfileKind, AppError> {
    match kind.trim() {
        PROFILE_KIND_CHARACTER => Ok(NormalizedProfileKind::Character),
        PROFILE_KIND_OBJECT => Ok(NormalizedProfileKind::Object),
        PROFILE_KIND_SCENE => Ok(NormalizedProfileKind::Scene),
        "" => Err(AppError::InvalidArgument(
            "profile_kind 不能为空".to_string(),
        )),
        other => Err(AppError::InvalidArgument(format!(
            "profile_kind 无效: {other}"
        ))),
    }
}

fn normalize_evidence_role(role: &str) -> Result<&str, AppError> {
    match role.trim() {
        EVIDENCE_ROLE_FIRST_SEEN => Ok(EVIDENCE_ROLE_FIRST_SEEN),
        EVIDENCE_ROLE_APPEARANCE => Ok(EVIDENCE_ROLE_APPEARANCE),
        EVIDENCE_ROLE_STATE_CHANGE => Ok(EVIDENCE_ROLE_STATE_CHANGE),
        EVIDENCE_ROLE_LOCATION => Ok(EVIDENCE_ROLE_LOCATION),
        EVIDENCE_ROLE_RELATIONSHIP => Ok(EVIDENCE_ROLE_RELATIONSHIP),
        EVIDENCE_ROLE_BEHAVIOR => Ok(EVIDENCE_ROLE_BEHAVIOR),
        "" => Err(AppError::InvalidArgument(
            "evidence_role 不能为空".to_string(),
        )),
        other => Err(AppError::InvalidArgument(format!(
            "evidence_role 无效: {other}"
        ))),
    }
}

fn to_i32(value: usize, field: &str) -> Result<i32, AppError> {
    i32::try_from(value)
        .map_err(|_| AppError::InvalidArgument(format!("{field} 超出 i32 范围: {value}")))
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{
        CreateDerivedProjectParams, DerivedChapterScope, PROJECT_KIND_VIDEO,
        PROJECT_TYPE_NOVEL_LONG, ProjectService,
    };
    use a4n_core::event::bridge::BroadcastEventBus;
    use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, QueryFilter};
    use serde_json::json;

    async fn setup_db() -> Arc<DatabaseConnection> {
        Arc::new(a4n_migration::test_support::connect_test_database().await)
    }

    fn project_service(db: Arc<DatabaseConnection>) -> ProjectService {
        ProjectService::new(db, Arc::new(BroadcastEventBus::new(16)))
    }

    #[test]
    fn text_anchor_prompt_values_hide_internal_anchor_fields() {
        let now = Utc::now().naive_utc();
        let chapter = chapter::Model {
            id: 42,
            project_id: 1,
            outline_id: None,
            chapter_number: 3,
            title: Some("第三章".to_string()),
            content: "他在旧书房抽屉里摸到一枚青铜钥匙。".to_string(),
            word_count: 20,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        };
        let anchor = text_anchor::Model {
            id: 99,
            project_id: 1,
            source_chapter_id: Some(chapter.id),
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_kind: SOURCE_KIND_CHAPTER_CONTENT.to_string(),
            source_id: chapter.id,
            paragraph_index: 1,
            start_offset: 0,
            end_offset: 10,
            quote: "他在旧书房抽屉里摸到一枚青铜钥匙".to_string(),
            quote_hash: "hash".to_string(),
            source_version_hash: "source-version".to_string(),
            created_at: now,
        };

        let values = text_anchor_prompt_values(&[chapter], &[anchor]);
        let object = values[0].as_object().expect("prompt anchor object");

        assert_eq!(object.get("id").and_then(Value::as_i64), Some(99));
        assert_eq!(
            object.get("chapter_label").and_then(Value::as_str),
            Some("第3章")
        );
        assert_eq!(
            object.get("quote").and_then(Value::as_str),
            Some("他在旧书房抽屉里摸到一枚青铜钥匙")
        );
        assert!(object.get("source_chapter_id").is_none());
        assert!(object.get("start_offset").is_none());
        assert!(object.get("end_offset").is_none());
        assert!(object.get("quote_hash").is_none());
    }

    fn chapter_for_anchor_validation() -> chapter::Model {
        let now = Utc::now().naive_utc();
        chapter::Model {
            id: 42,
            project_id: 1,
            outline_id: None,
            chapter_number: 3,
            title: Some("第三章".to_string()),
            content: "他在旧书房抽屉里摸到一枚青铜钥匙。".to_string(),
            word_count: 20,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn anchor_for_validation(
        chapter: &chapter::Model,
        source_version_hash: &str,
    ) -> text_anchor::Model {
        let now = Utc::now().naive_utc();
        text_anchor::Model {
            id: 99,
            project_id: chapter.project_id,
            source_chapter_id: Some(chapter.id),
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_kind: SOURCE_KIND_CHAPTER_CONTENT.to_string(),
            source_id: chapter.id,
            paragraph_index: 1,
            start_offset: 0,
            end_offset: 10,
            quote: "他在旧书房抽屉里摸到一枚青铜钥匙".to_string(),
            quote_hash: "hash".to_string(),
            source_version_hash: source_version_hash.to_string(),
            created_at: now,
        }
    }

    #[test]
    fn validate_current_chapter_content_anchors_rejects_missing_anchors() {
        let chapter = chapter_for_anchor_validation();
        let source_hash = hash_text(&chapter.content);

        let err = validate_current_chapter_content_anchors(&chapter, Vec::new(), &source_hash)
            .expect_err("missing anchors should fail");

        assert!(matches!(err, AppError::InvalidState { current, operation }
                if current.contains("缺少当前正文文本锚点") && operation == "预览模型请求"));
    }

    #[test]
    fn validate_current_chapter_content_anchors_rejects_stale_anchors() {
        let chapter = chapter_for_anchor_validation();
        let source_hash = hash_text(&chapter.content);
        let stale_anchor = anchor_for_validation(&chapter, "old-hash");

        let err =
            validate_current_chapter_content_anchors(&chapter, vec![stale_anchor], &source_hash)
                .expect_err("stale anchors should fail");

        assert!(matches!(err, AppError::InvalidState { current, operation }
                if current.contains("正文文本锚点已过期") && operation == "预览模型请求"));
    }

    #[test]
    fn validate_current_chapter_content_anchors_accepts_matching_anchors() {
        let chapter = chapter_for_anchor_validation();
        let source_hash = hash_text(&chapter.content);
        let anchor = anchor_for_validation(&chapter, &source_hash);

        let result =
            validate_current_chapter_content_anchors(&chapter, vec![anchor], &source_hash).unwrap();

        assert_eq!(result.len(), 1);
    }

    async fn insert_chapter(
        db: &DatabaseConnection,
        project_id: i64,
        chapter_number: i32,
        content: &str,
    ) -> chapter::Model {
        let now = Utc::now().naive_utc();
        chapter::ActiveModel {
            project_id: Set(project_id),
            outline_id: Set(None),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("第{chapter_number}章"))),
            content: Set(content.to_string()),
            word_count: Set(content.chars().count() as i32),
            status: Set(chapter::STATUS_IMPORTED.to_string()),
            summary: Set(Some("章节摘要".to_string())),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn insert_object_profile(
        db: &DatabaseConnection,
        project_id: i64,
    ) -> object_profile::Model {
        let now = Utc::now().naive_utc();
        object_profile::ActiveModel {
            project_id: Set(project_id),
            name: Set("青铜钥匙".to_string()),
            normalized_name: Set("青铜钥匙".to_string()),
            aliases_json: Set(json!([])),
            object_type: Set("token".to_string()),
            importance: Set("primary".to_string()),
            visual_description: Set("旧青铜钥匙".to_string()),
            material_and_state: Set("氧化青铜".to_string()),
            story_usage: Set("开启旧门".to_string()),
            visual_usage: Set("保持钥匙作为开门动作道具的画面连续性".to_string()),
            generation_usage: Set("约束旧青铜钥匙外观和氧化材质".to_string()),
            location_context: Set(Some("抽屉".to_string())),
            first_seen_chapter: Set(1),
            last_seen_chapter: Set(1),
            evidence_json: Set(json!(["他在抽屉里摸到一枚青铜钥匙"])),
            image_prompt_seed: Set("一枚旧青铜钥匙".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn rebuild_chapter_content_anchors_persists_and_replaces_existing_rows() {
        let db = setup_db().await;
        let projects = project_service(db.clone());
        let anchors = TextAnchorService::new(db.clone());
        let project = projects
            .create(1, "根文本", "", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        let chapter = insert_chapter(
            db.as_ref(),
            project.id,
            1,
            "他走进旧书房，抽屉里有一枚青铜钥匙。钥匙很冷。",
        )
        .await;

        let first = anchors
            .rebuild_chapter_content_anchors(project.id, chapter.id)
            .await
            .unwrap();
        let second = anchors
            .rebuild_chapter_content_anchors(project.id, chapter.id)
            .await
            .unwrap();

        assert_eq!(first.len(), 3);
        assert_eq!(second.len(), 3);
        let stored_count = text_anchor::Entity::find()
            .filter(text_anchor::Column::ProjectId.eq(project.id))
            .count(db.as_ref())
            .await
            .unwrap();
        assert_eq!(stored_count, 3);
        assert!(
            second
                .iter()
                .any(|anchor| anchor.quote == "抽屉里有一枚青铜钥匙")
        );
    }

    #[tokio::test]
    async fn rebuild_snapshot_chapter_content_anchors_binds_snapshot_identity() {
        let db = setup_db().await;
        let projects = project_service(db.clone());
        let anchors = TextAnchorService::new(db.clone());
        let root = projects
            .create(1, "根文本", "", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        insert_chapter(db.as_ref(), root.id, 1, "他推开门，风从门缝里灌进来。").await;
        let derived = projects
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_VIDEO.to_string(),
                    title: "视频项目".to_string(),
                    description: "".to_string(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .unwrap();
        let snapshot_id = derived.source_snapshot_id.unwrap();
        let snapshot_chapter = source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();

        let result = anchors
            .rebuild_snapshot_chapter_content_anchors(root.id, snapshot_chapter.id)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert!(
            result
                .iter()
                .all(|anchor| anchor.source_snapshot_id == Some(snapshot_id))
        );
        assert!(
            result
                .iter()
                .all(|anchor| anchor.source_snapshot_chapter_id == Some(snapshot_chapter.id))
        );
    }

    #[tokio::test]
    async fn validate_anchor_refs_rejects_cross_project_anchor() {
        let db = setup_db().await;
        let projects = project_service(db.clone());
        let anchors = TextAnchorService::new(db.clone());
        let first_project = projects
            .create(1, "根文本 A", "", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        let second_project = projects
            .create(1, "根文本 B", "", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        let first_chapter =
            insert_chapter(db.as_ref(), first_project.id, 1, "他看见青铜钥匙。").await;
        let anchor = anchors
            .rebuild_chapter_content_anchors(first_project.id, first_chapter.id)
            .await
            .unwrap()
            .remove(0);

        let err = anchors
            .validate_anchor_refs(second_project.id, None, &[], &[anchor.id])
            .await
            .expect_err("cross project anchor should fail");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("不属于文本根项目"))
        );
    }

    #[tokio::test]
    async fn replace_profile_evidence_persists_anchor_binding() {
        let db = setup_db().await;
        let projects = project_service(db.clone());
        let anchors = TextAnchorService::new(db.clone());
        let project = projects
            .create(1, "根文本", "", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        let chapter =
            insert_chapter(db.as_ref(), project.id, 1, "他在抽屉里摸到一枚青铜钥匙。").await;
        let object = insert_object_profile(db.as_ref(), project.id).await;
        let anchor = anchors
            .rebuild_chapter_content_anchors(project.id, chapter.id)
            .await
            .unwrap()
            .remove(0);

        let evidence = anchors
            .replace_profile_evidence(
                project.id,
                None,
                PROFILE_KIND_OBJECT,
                object.id,
                vec![ProfileEvidenceInput {
                    text_anchor_id: anchor.id,
                    evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
                }],
            )
            .await
            .unwrap();

        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].profile_kind, PROFILE_KIND_OBJECT);
        assert_eq!(evidence[0].profile_id, object.id);
        assert_eq!(evidence[0].text_anchor_id, anchor.id);
    }

    #[tokio::test]
    async fn rebuild_chapter_content_anchors_rejects_derived_project() {
        let db = setup_db().await;
        let projects = project_service(db.clone());
        let anchors = TextAnchorService::new(db.clone());
        let root = projects
            .create(1, "根文本", "", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        insert_chapter(db.as_ref(), root.id, 1, "他推开门。").await;
        let derived = projects
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_VIDEO.to_string(),
                    title: "视频项目".to_string(),
                    description: "".to_string(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .unwrap();

        let err = anchors
            .rebuild_chapter_content_anchors(derived.id, 1)
            .await
            .expect_err("derived project should fail");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("不是文本根项目"))
        );
    }
}
