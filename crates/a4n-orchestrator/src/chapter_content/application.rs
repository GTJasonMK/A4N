use std::collections::BTreeSet;

use a4n_core::{
    error::AppError,
    models::{chapter::chapter, character::protagonist_profile},
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

    async fn sync_chapter_rag_records(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
    ) -> Result<(), AppError>;
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

    pub async fn postprocess_chapter_derived_data_validated(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        chapter_id: i64,
        cleanup_foreshadowing: bool,
    ) -> Result<(), AppError> {
        let chapter = self.chapter_by_id_for_analysis(chapter_id).await?;
        let lock = self.chapter_content_sync_lock(chapter.project_id, chapter.chapter_number);
        let _guard = lock.lock_owned().await;
        let (_, run) = self
            .begin_existing_chapter_content_sync(
                chapter_id,
                cleanup_foreshadowing,
                true,
                |artifacts| host_port.validate_chapter_cleanup_artifacts(artifacts),
            )
            .await?;
        self.settle_chapter_content_sync_run(host_port, &run)
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
        let lock = self.chapter_content_sync_lock(chapter.project_id, chapter.chapter_number);
        let _guard = lock.lock_owned().await;
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
        let (updated, run) = self
            .save_chapter_content_edit_with_sync_plan(chapter_id, content, title, |artifacts| {
                host_port.validate_chapter_cleanup_artifacts(artifacts)
            })
            .await?;
        if let Err(error) = self.settle_chapter_content_sync_run(host_port, &run).await {
            tracing::warn!(
                chapter_id = updated.id,
                workflow_run_id = run.id,
                error = %error,
                "章节正文已保存，派生资料同步待恢复"
            );
        }
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
    ) -> Result<chapter::Model, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.ensure_text_project_validated(project_id, "导入章节正文")
            .await?;
        let lock = self.chapter_content_sync_lock(project_id, chapter_number);
        let _guard = lock.lock_owned().await;
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
        let (chapter, run) = self
            .import_chapter_content_with_sync_plan(
                project_id,
                chapter_number,
                content,
                title,
                "导入覆盖正文",
                |artifacts| host_port.validate_chapter_cleanup_artifacts(artifacts),
            )
            .await?;
        if let Err(error) = self.settle_chapter_content_sync_run(host_port, &run).await {
            tracing::warn!(
                chapter_id = chapter.id,
                workflow_run_id = run.id,
                error = %error,
                "导入正文已保存，派生资料同步待恢复"
            );
        }
        self.chapter_by_id_for_analysis(chapter.id).await
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
