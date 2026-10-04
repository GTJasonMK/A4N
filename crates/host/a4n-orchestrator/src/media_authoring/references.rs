use a4n_api_contract::media_authoring::*;
use a4n_domain::{
    chapter::ResolvedChapterSource, collaboration::ProjectAccessRole,
    shooting_script::ChapterShootingScriptReferenceRecord,
};
use a4n_error::AppError;

use super::{authoring_source, records};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub(super) async fn list_video_references(
        &self,
        user_id: i64,
        input: MediaAuthoringInput,
    ) -> Result<Vec<VideoScriptReferenceCandidate>, AppError> {
        let (_, _, sources) = self
            .authoring_sources(
                user_id,
                input.project_id,
                input.chapter_id,
                ProjectAccessRole::Viewer,
            )
            .await?;
        let root = sources
            .first()
            .ok_or_else(|| AppError::NotFound("项目没有原文章节".into()))?
            .root_project_id;
        let scripts =
            records::video_reference_scripts(self.database.executor(), root, input.chapter_id)
                .await?;
        let mut candidates = Vec::new();
        for (project, script) in scripts {
            let Some(current) = sources
                .iter()
                .find(|source| source.source_chapter_id == script.source_chapter_id)
            else {
                continue;
            };
            match self
                .verify_project_permission(project.id, user_id, ProjectAccessRole::Viewer)
                .await
            {
                Ok(_) => {}
                Err(AppError::Forbidden(_) | AppError::NotFound(_)) => continue,
                Err(error) => return Err(error),
            }
            let source = self.reference_script_source(&script).await?;
            candidates.push(VideoScriptReferenceCandidate {
                selection: VideoScriptSelection {
                    video_project_id: project.id,
                    script_id: script.id,
                    revision: script.revision,
                },
                project_title: project.title,
                chapter_number: source.chapter_number,
                same_source_content: current.content_sha256 == source.content_sha256,
                source: authoring_source(&source)?,
            });
        }
        Ok(candidates)
    }

    pub(super) async fn freeze_video_reference(
        &self,
        user_id: i64,
        source: &ResolvedChapterSource,
        target: AdaptationTarget,
        selection: &VideoScriptSelection,
    ) -> Result<VideoAdaptationReference, AppError> {
        if selection.revision <= 0 || selection.script_id <= 0 || selection.video_project_id <= 0 {
            return Err(AppError::InvalidArgument("视频参考身份必须是正整数".into()));
        }
        self.verify_project_permission(
            selection.video_project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let project = self
            .project_by_id(selection.video_project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("参考视频项目不存在".into()))?;
        if project.project_kind != "video"
            || project.root_project_id != Some(source.root_project_id)
        {
            return Err(AppError::InvalidArgument(
                "只能参考同一原作的视频派生项目".into(),
            ));
        }
        let record = records::video_reference_script(
            self.database.executor(),
            selection.video_project_id,
            selection.script_id,
        )
        .await?
        .ok_or_else(|| AppError::NotFound("视频参考剧本不存在".into()))?;
        if record.revision != selection.revision
            || record.source_chapter_id != source.source_chapter_id
        {
            return Err(AppError::InvalidArgument(
                "参考剧本已更新或章节不匹配，请重新选择版本".into(),
            ));
        }
        let reference_source = self.reference_script_source(&record).await?;
        let script: a4n_domain::shooting_script::ChapterShootingScriptV2 =
            serde_json::from_value(record.script_json)?;
        let hash = a4n_domain::shooting_script::compat::serialized_sha256(&script)?;
        if hash != record.script_sha256 {
            return Err(AppError::InvalidArgument("参考剧本内容哈希不一致".into()));
        }
        let (adaptation, _) =
            a4n_domain::media_authoring::legacy::adaptation_from_shooting_script(&script, target)
                .map_err(AppError::InvalidArgument)?;
        Ok(VideoAdaptationReference {
            video_project_id: Some(project.id),
            project_title: project.title,
            source: authoring_source(&reference_source)?,
            script_id: record.id,
            script_revision: record.revision,
            script_sha256: hash,
            units: adaptation.units,
        })
    }

    async fn reference_script_source(
        &self,
        record: &ChapterShootingScriptReferenceRecord,
    ) -> Result<ResolvedChapterSource, AppError> {
        let snapshot_id = record
            .source_snapshot_id
            .ok_or_else(|| AppError::InvalidArgument("参考剧本缺少冻结原文".into()))?;
        let mut sources = a4n_db::project::resolve_snapshot_sources(
            self.database.executor(),
            record.project_id,
            record.root_project_id,
            snapshot_id,
            &[record.source_chapter_id],
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)?;
        let source = sources
            .pop()
            .ok_or_else(|| AppError::NotFound("参考剧本的原文章节不存在".into()))?;
        if source.source_snapshot_chapter_id != record.source_snapshot_chapter_id
            || source.content_sha256 != record.source_content_sha256
        {
            return Err(AppError::InvalidArgument("参考剧本原文身份不一致".into()));
        }
        Ok(source)
    }

    pub(super) async fn video_reference_status(
        &self,
        user_id: i64,
        reference: Option<&VideoAdaptationReference>,
    ) -> Result<Option<VideoReferenceStatus>, AppError> {
        let Some(reference) = reference else {
            return Ok(None);
        };
        let Some(project_id) = reference.video_project_id else {
            return Ok(Some(VideoReferenceStatus::Unavailable));
        };
        match self
            .verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await
        {
            Ok(_) => {}
            Err(AppError::Forbidden(_) | AppError::NotFound(_)) => {
                return Ok(Some(VideoReferenceStatus::Unavailable));
            }
            Err(error) => return Err(error),
        }
        let record = records::video_reference_script(
            self.database.executor(),
            project_id,
            reference.script_id,
        )
        .await?;
        Ok(Some(match record {
            None => VideoReferenceStatus::Unavailable,
            Some(record)
                if record.revision == reference.script_revision
                    && record.script_sha256 == reference.script_sha256 =>
            {
                VideoReferenceStatus::Current
            }
            Some(_) => VideoReferenceStatus::Updated,
        }))
    }
}
