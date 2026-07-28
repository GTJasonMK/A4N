use std::collections::BTreeSet;

use a4n_core::{
    error::AppError,
    models::{
        chapter::{chapter, chapter_outline},
        character::protagonist_profile,
    },
};
use async_trait::async_trait;

use crate::{
    OrchestratorContext,
    project::{ChapterDerivedCleanupArtifacts, chapter_cleanup_requires_snapshot},
};

use super::{
    ensure_chapter_content_not_empty, ensure_chapter_not_generating, ensure_import_chapter_number,
    ensure_text_project_kind,
};

#[async_trait]
pub trait ChapterDerivedCleanupHostPort: Send + Sync {
    fn validate_chapter_cleanup_artifacts(
        &self,
        artifacts: &ChapterDerivedCleanupArtifacts,
    ) -> Result<(), AppError>;

    async fn finish_chapter_derived_cleanup(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        artifacts: ChapterDerivedCleanupArtifacts,
    ) -> Result<(), AppError>;
}

pub trait ChapterOutlineCleanupHostPort: ChapterDerivedCleanupHostPort {
    fn active_generation_chapter(&self, project_id: i64, chapter_numbers: &[i32]) -> Option<i32>;
}

impl OrchestratorContext {
    pub async fn preflight_chapter_content_change_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), AppError> {
        let profiles = self.protagonist_profiles_for_project(project_id).await?;
        let snapshot_chapter = chapter_number - 1;
        let snapshot_profile_ids = profiles
            .iter()
            .filter(|profile| chapter_cleanup_requires_snapshot(profile, chapter_number))
            .map(|profile| profile.id)
            .collect::<Vec<_>>();
        if snapshot_profile_ids.is_empty() {
            return Ok(());
        }

        let existing_snapshot_profile_ids = self
            .snapshot_profile_ids_for_chapter(&snapshot_profile_ids, snapshot_chapter)
            .await?
            .into_iter()
            .collect::<BTreeSet<_>>();
        let missing_names = profiles_missing_cleanup_snapshots(
            &profiles,
            &existing_snapshot_profile_ids,
            chapter_number,
        );
        if missing_names.is_empty() {
            return Ok(());
        }

        Err(AppError::InvalidState {
            current: "章节派生数据无法清理".to_string(),
            operation: format!(
                "保存章节正文前预检：以下角色缺少第 {snapshot_chapter} 章快照，无法在重写第 {chapter_number} 章前回滚档案状态：{}",
                missing_names.join("、")
            ),
        })
    }

    pub async fn postprocess_chapter_after_content_change_validated(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let before = self.chapter_by_id_for_analysis(chapter_id).await?;
        let artifacts = self
            .cleanup_chapter_derived_data_record(
                before.project_id,
                before.id,
                before.chapter_number,
                false,
                |artifacts| host_port.validate_chapter_cleanup_artifacts(artifacts),
            )
            .await
            .map_err(|error| {
                AppError::Internal(anyhow::anyhow!(
                    "章节后处理停止：清理章节派生数据失败: {error}"
                ))
            })?;
        host_port
            .finish_chapter_derived_cleanup(
                before.project_id,
                before.id,
                before.chapter_number,
                artifacts,
            )
            .await?;

        if !before.has_available_content() {
            self.refresh_project_current_chapter(before.project_id)
                .await?;
            return Ok(());
        }

        self.process_chapter(before.id).await?;
        let chapter = self.chapter_by_id_for_analysis(before.id).await?;
        self.refresh_project_current_chapter(chapter.project_id)
            .await?;
        self.sync_project_status_from_artifacts(chapter.project_id)
            .await?;
        Ok(())
    }

    pub async fn update_chapter_content_validated(
        &self,
        user_id: i64,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        chapter_id: i64,
        content: String,
        title: Option<String>,
    ) -> Result<chapter::Model, AppError> {
        let chapter = self.chapter_by_id_for_analysis(chapter_id).await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.ensure_text_project_validated(chapter.project_id, "更新章节正文")
            .await?;
        ensure_chapter_not_generating(&chapter, "更新章节正文")?;
        ensure_chapter_content_not_empty(&content)?;
        self.preflight_chapter_content_change_validated(chapter.project_id, chapter.chapter_number)
            .await?;
        let updated = self
            .save_chapter_content_edit_validated(chapter_id, content, title)
            .await?;
        self.postprocess_chapter_after_content_change_validated(host_port, updated.id)
            .await?;
        self.chapter_by_id_for_analysis(updated.id).await
    }

    pub async fn import_chapter_validated(
        &self,
        user_id: i64,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        project_id: i64,
        chapter_number: i32,
        content: String,
        title: Option<String>,
    ) -> Result<(chapter::Model, i32), AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.ensure_text_project_validated(project_id, "导入章节正文")
            .await?;
        ensure_import_chapter_number(chapter_number)?;
        ensure_chapter_content_not_empty(&content)?;
        self.preflight_chapter_content_change_validated(project_id, chapter_number)
            .await?;
        let (chapter, word_count) = self
            .import_chapter_content_validated(
                project_id,
                chapter_number,
                content,
                title,
                "导入覆盖正文",
            )
            .await?;
        self.postprocess_chapter_after_content_change_validated(host_port, chapter.id)
            .await?;
        Ok((chapter, word_count))
    }

    pub async fn delete_latest_chapter_outlines_validated(
        &self,
        host_port: &dyn ChapterOutlineCleanupHostPort,
        project_id: i64,
        count: u64,
    ) -> Result<(), AppError> {
        let outlines = self.chapter_outlines_for_project(project_id).await?;
        let to_delete = latest_chapter_outlines_to_delete(&outlines, count)?;
        let outline_ids = to_delete
            .iter()
            .map(|outline| outline.id)
            .collect::<Vec<_>>();
        let chapter_numbers = to_delete
            .iter()
            .map(|outline| outline.chapter_number)
            .collect::<Vec<_>>();
        if let Some(chapter_number) =
            host_port.active_generation_chapter(project_id, &chapter_numbers)
        {
            return Err(generating_chapter_delete_error(chapter_number));
        }
        let chapters = self
            .chapters_for_project_numbers(project_id, &chapter_numbers)
            .await?;
        if let Some(chapter) = chapters
            .iter()
            .find(|chapter| chapter.status == "generating")
        {
            return Err(generating_chapter_delete_error(chapter.chapter_number));
        }
        let cleanup_results = self
            .delete_chapter_content_records_and_outlines_record(
                project_id,
                chapters,
                &outline_ids,
                |cleanup_results| {
                    for cleanup in cleanup_results {
                        host_port
                            .validate_chapter_cleanup_artifacts(&cleanup.artifacts)
                            .map_err(|error| AppError::InvalidState {
                                current: format!(
                                    "chapter_number={}：{error}",
                                    cleanup.chapter_number
                                ),
                                operation: "清理章节派生媒体文件".to_string(),
                            })?;
                    }
                    Ok(())
                },
            )
            .await?;
        for cleanup in cleanup_results {
            host_port
                .finish_chapter_derived_cleanup(
                    project_id,
                    cleanup.chapter_id,
                    cleanup.chapter_number,
                    cleanup.artifacts,
                )
                .await?;
        }
        self.refresh_project_current_chapter(project_id).await
    }

    async fn ensure_text_project_validated(
        &self,
        project_id: i64,
        operation: &str,
    ) -> Result<(), AppError> {
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        ensure_text_project_kind(&project.project_kind, operation)
    }
}

fn latest_chapter_outlines_to_delete(
    outlines: &[chapter_outline::Model],
    count: u64,
) -> Result<Vec<chapter_outline::Model>, AppError> {
    if count == 0 {
        return Err(AppError::InvalidArgument(
            "删除章纲数量必须大于 0".to_string(),
        ));
    }
    if count as usize > outlines.len() {
        return Err(AppError::InvalidArgument(format!(
            "删除章纲数量超过现有章纲数量：count={count}, total={}",
            outlines.len()
        )));
    }
    let mut seen = BTreeSet::new();
    for outline in outlines {
        if outline.chapter_number <= 0 {
            return Err(AppError::InvalidState {
                current: outline.chapter_number.to_string(),
                operation: "已有章节大纲章节号无效，请先清理章纲状态后再删除".to_string(),
            });
        }
        if !seen.insert(outline.chapter_number) {
            return Err(AppError::InvalidState {
                current: outline.chapter_number.to_string(),
                operation: format!(
                    "第 {} 章已有重复章节大纲，请先清理章纲状态后再删除",
                    outline.chapter_number
                ),
            });
        }
    }
    let mut sorted = outlines.to_vec();
    sorted.sort_by_key(|outline| (outline.chapter_number, outline.id));
    Ok(sorted.into_iter().rev().take(count as usize).collect())
}

fn generating_chapter_delete_error(chapter_number: i32) -> AppError {
    AppError::InvalidState {
        current: "generating".to_string(),
        operation: format!("第 {chapter_number} 章正在生成中，请等待当前生成完成后再删除章纲"),
    }
}

fn profiles_missing_cleanup_snapshots(
    profiles: &[protagonist_profile::Model],
    existing_snapshot_profile_ids: &BTreeSet<i64>,
    chapter_number: i32,
) -> Vec<String> {
    profiles
        .iter()
        .filter(|profile| chapter_cleanup_requires_snapshot(profile, chapter_number))
        .filter(|profile| !existing_snapshot_profile_ids.contains(&profile.id))
        .map(|profile| profile.name.clone())
        .collect()
}
