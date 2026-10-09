//! 项目族导出：选择校验、快照覆盖校验与 manifest 组装（不含磁盘媒体读取）。

use std::collections::HashSet;

use a4n_db::project::ProjectFamilyExportRows;
use a4n_domain::clear_canvas_transfer_runtime_identity;
use a4n_domain::project::ProjectRecord;
use a4n_domain::video_prompt::STATUS_RUNNING;
#[cfg(test)]
use a4n_domain::video_prompt::{ChapterVideoPromptRecord, STATUS_PENDING};
use chrono::Utc;

use super::{PROJECT_FAMILY_PACKAGE_VERSION, ProjectFamilyPackage};
use crate::project::{MediaFileEntry, required_path_extension};

/// 校验根项目与派生项目归属同一用户且挂在正确 root 下。
fn validate_export_same_user(
    root: &ProjectRecord,
    derived_projects: &[ProjectRecord],
) -> Result<(), String> {
    for project in derived_projects {
        if project.user_id != root.user_id {
            return Err(format!(
                "派生项目所属用户与根文本项目不一致: {}",
                project.id
            ));
        }
        if project.root_project_id != Some(root.id) {
            return Err(format!("派生项目不属于根文本项目: {}", project.id));
        }
    }
    Ok(())
}

/// 校验导出行中的源快照覆盖派生引用。
fn validate_export_snapshot_coverage(
    expected_ids: &[i64],
    snapshots: &[a4n_domain::project::ProjectFamilySourceSnapshotRecord],
) -> Result<(), String> {
    let found = snapshots
        .iter()
        .map(|snapshot| snapshot.id)
        .collect::<HashSet<_>>();
    for id in expected_ids {
        if !found.contains(id) {
            return Err(format!("派生项目引用的源文本快照不存在: {id}"));
        }
    }
    Ok(())
}

/// 组装导出 manifest（`media_files` 由 Host 根据磁盘填充）。
pub(crate) fn assemble_project_family_export_package(
    root: ProjectRecord,
    derived_projects: Vec<ProjectRecord>,
    mode: &str,
    mut export_rows: ProjectFamilyExportRows,
    media_files: Vec<MediaFileEntry>,
) -> Result<ProjectFamilyPackage, String> {
    validate_export_same_user(&root, &derived_projects)?;
    validate_export_snapshot_coverage(&export_rows.snapshot_ids, &export_rows.source_snapshots)?;
    prepare_rows_for_export(&mut export_rows)?;

    let mut projects = Vec::with_capacity(1 + derived_projects.len());
    projects.push(root.clone());
    projects.extend(derived_projects);

    Ok(ProjectFamilyPackage {
        version: PROJECT_FAMILY_PACKAGE_VERSION.to_string(),
        exported_at: Utc::now().naive_utc().to_string(),
        mode: mode.to_string(),
        root_project_id: root.id,
        projects,
        text_core: export_rows.text_core,
        source_snapshots: export_rows.source_snapshots,
        source_snapshot_chapters: export_rows.source_snapshot_chapters,
        source_snapshot_outlines: export_rows.source_snapshot_outlines,
        source_snapshot_profiles: export_rows.source_snapshot_profiles,
        text_anchors: export_rows.text_anchors,
        profile_evidence: export_rows.profile_evidence,
        chapter_visual_plans: export_rows.chapter_visual_plans,
        chapter_shooting_scripts: export_rows.chapter_shooting_scripts,
        authoring_documents: export_rows.authoring_documents,
        authoring_receipts: export_rows.authoring_receipts,
        character_portraits: export_rows.character_portraits,
        generated_images: export_rows.generated_images,
        chapter_manga_prompts: export_rows.chapter_manga_prompts,
        manga_pipeline_artifacts: export_rows.manga_pipeline_artifacts,
        chapter_video_prompts: export_rows.chapter_video_prompts,
        generated_videos: export_rows.generated_videos,
        generation_specs: export_rows.generation_specs,
        generation_spec_text_anchors: export_rows.generation_spec_text_anchors,
        generation_spec_entity_refs: export_rows.generation_spec_entity_refs,
        generation_spec_media_refs: export_rows.generation_spec_media_refs,
        prompt_artifacts: export_rows.prompt_artifacts,
        media_assets: export_rows.media_assets,
        media_provenance: export_rows.media_provenance,
        media_bindings: export_rows.media_bindings,
        galgame_scripts: export_rows.galgame_scripts,
        galgame_scenes: export_rows.galgame_scenes,
        galgame_lines: export_rows.galgame_lines,
        galgame_choices: export_rows.galgame_choices,
        video_segments: export_rows.video_segments,
        assembly_timelines: export_rows.assembly_timelines,
        assembly_clips: export_rows.assembly_clips,
        source_sync_runs: export_rows.source_sync_runs,
        source_sync_conflicts: export_rows.source_sync_conflicts,
        source_sync_decisions: export_rows.source_sync_decisions,
        media_files,
    })
}

fn prepare_rows_for_export(rows: &mut ProjectFamilyExportRows) -> Result<(), String> {
    for script in &mut rows.chapter_shooting_scripts {
        script.workflow_run_id = None;
    }
    for image in &mut rows.generated_images {
        if let Some(metadata) = image.metadata_json.as_mut() {
            clear_canvas_transfer_runtime_identity(metadata);
        }
    }
    for prompt in &mut rows.chapter_video_prompts {
        if prompt.status == STATUS_RUNNING {
            return Err(format!(
                "项目族不能导出运行中的章节视频提示词: chapter_video_prompt_id={}",
                prompt.id
            ));
        }
        if prompt.timeline_item_id.is_some() {
            return Err(format!(
                "项目族不能导出已绑定时间线项的章节视频提示词: chapter_video_prompt_id={}",
                prompt.id
            ));
        }
        if let Some(metadata) = prompt.metadata_json.as_mut() {
            clear_canvas_transfer_runtime_identity(metadata);
        }
    }
    for video in &mut rows.generated_videos {
        if !a4n_domain::video_workflow::is_terminal_video_status(&video.status) {
            return Err(format!(
                "项目族不能导出非终态生成视频: generated_video_id={}, status={}",
                video.id, video.status
            ));
        }
        if video.timeline_item_id.is_some() {
            return Err(format!(
                "项目族不能导出已绑定时间线项的生成视频: generated_video_id={}",
                video.id
            ));
        }
        video.provider_job_id = None;
        if let Some(metadata) = video.metadata_json.as_mut() {
            clear_canvas_transfer_runtime_identity(metadata);
        }
    }
    for spec in &mut rows.generation_specs {
        clear_canvas_transfer_runtime_identity(&mut spec.params_json);
        if spec.source_context_kind.as_deref()
            == Some(a4n_domain::generation::SOURCE_CONTEXT_FREEFORM)
        {
            spec.source_version_hash =
                a4n_domain::generation::freeform_source_version_hash(&spec.params_json)
                    .ok_or_else(|| format!("画布生成意图缺少 source_material: {}", spec.id))?;
        }
    }
    for provenance in &mut rows.media_provenance {
        provenance.provider_job_id = None;
        if let Some(request) = provenance.request_snapshot_json.as_mut() {
            clear_canvas_transfer_runtime_identity(request);
        }
    }
    Ok(())
}

/// 导出媒体包内相对路径：`media/{owner_kind}/{owner_id}/{field}.{ext}`。
pub(crate) fn family_media_package_path(
    owner_kind: &str,
    owner_id: i64,
    field: &str,
    original_path: &str,
) -> Result<String, String> {
    let extension = required_path_extension(original_path, "媒体文件")?;
    Ok(format!("media/{owner_kind}/{owner_id}/{field}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn project(id: i64, user_id: i64, root: Option<i64>) -> ProjectRecord {
        ProjectRecord {
            id,
            user_id,
            title: format!("p{id}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
            project_kind: if root.is_some() {
                "video".to_string()
            } else {
                "text".to_string()
            },
            root_project_id: root,
            source_snapshot_id: root.map(|_| 1000 + id),
            source_sync_status: "current".to_string(),
            status: "draft".to_string(),
            lifecycle_gate: "active".to_string(),
            target_chapters: 20,
            current_chapter: 0,
            total_chapters: Some(20),
            chapters_per_part: Some(10),
            avatar_svg: None,
            avatar_animal: None,
            raw_blueprint: None,
            import_analysis_progress: None,
            export_config: None,
            created_at: dt(),
            updated_at: dt(),
        }
    }

    #[test]
    fn package_project_keeps_internal_lifecycle_gate_out_of_manifest() {
        let package = assemble_project_family_export_package(
            project(1, 7, None),
            Vec::new(),
            "project_family",
            ProjectFamilyExportRows::default(),
            Vec::new(),
        )
        .expect("项目族导出应成功");
        let manifest = serde_json::to_value(package).expect("项目族包应可序列化");

        assert!(manifest["projects"][0].get("lifecycle_gate").is_none());
        serde_json::from_value::<ProjectFamilyPackage>(manifest)
            .expect("不含内部状态的项目族包应可导入");
    }

    #[test]
    fn validate_export_same_user_rejects_mismatched_user() {
        let root = project(1, 10, None);
        let derived = vec![project(2, 11, Some(1))];
        let err = validate_export_same_user(&root, &derived).unwrap_err();
        assert!(err.contains("所属用户"));
    }

    #[test]
    fn validate_export_same_user_rejects_wrong_root() {
        let root = project(1, 10, None);
        let derived = vec![project(2, 10, Some(99))];
        let err = validate_export_same_user(&root, &derived).unwrap_err();
        assert!(err.contains("不属于根文本项目"));
    }

    #[test]
    fn family_media_package_path_requires_extension() {
        let err = family_media_package_path("media_asset", 1, "file_path", "noext").unwrap_err();
        assert!(err.contains("扩展名"));
        assert_eq!(
            family_media_package_path("media_asset", 3, "thumbnail_path", "a/b.png").unwrap(),
            "media/media_asset/3/thumbnail_path.png"
        );
    }

    #[test]
    fn prepare_rows_rejects_active_or_timeline_video_rows() {
        let mut rows = ProjectFamilyExportRows {
            chapter_video_prompts: vec![video_prompt(STATUS_RUNNING)],
            ..Default::default()
        };
        assert!(prepare_rows_for_export(&mut rows).is_err());

        rows.chapter_video_prompts[0].status = STATUS_PENDING.to_string();
        rows.chapter_video_prompts[0].timeline_item_id = Some(7);
        assert!(prepare_rows_for_export(&mut rows).is_err());

        rows.chapter_video_prompts.clear();
        rows.generated_videos = vec![generated_video(
            a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED,
        )];
        assert!(prepare_rows_for_export(&mut rows).is_err());
    }

    #[test]
    fn prepare_rows_clears_video_runtime_identity() {
        let mut rows = ProjectFamilyExportRows {
            generated_videos: vec![generated_video(
                a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED,
            )],
            ..Default::default()
        };

        prepare_rows_for_export(&mut rows).expect("terminal video should export");

        let video = &rows.generated_videos[0];
        assert_eq!(video.provider_job_id, None);
        let metadata = video.metadata_json.as_ref().unwrap();
        assert!(metadata.get("workflow_run_id").is_none());
        assert!(metadata["nested"].get("config_id").is_none());
    }

    fn video_prompt(status: &str) -> ChapterVideoPromptRecord {
        ChapterVideoPromptRecord {
            id: 10,
            project_id: 2,
            chapter_id: 3,
            style: None,
            prompt_type: "video_prompt".to_string(),
            workflow_mode: "boundary_frames".to_string(),
            shot_number: 1,
            title: None,
            scene_summary: None,
            visual_prompt: "prompt".to_string(),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: None,
            status: status.to_string(),
            generated_video_id: None,
            timeline_item_id: None,
            metadata_json: None,
            created_at: dt(),
            updated_at: dt(),
        }
    }

    fn generated_video(status: &str) -> a4n_domain::project::ProjectFamilyGeneratedVideoRecord {
        a4n_domain::project::ProjectFamilyGeneratedVideoRecord {
            id: 20,
            project_id: 2,
            chapter_id: Some(3),
            video_type: "chapter_video".to_string(),
            workflow_mode: Some("boundary_frames".to_string()),
            file_path: Some("video.mp4".to_string()),
            thumbnail_path: None,
            prompt: "prompt".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 6.0,
            fps: 24,
            file_size: Some(3),
            source_video_id: None,
            source_image_id: None,
            status: status.to_string(),
            provider_job_id: Some("job-20".to_string()),
            error_message: None,
            timeline_item_id: None,
            metadata_json: Some(serde_json::json!({
                "workflow_run_id": 9,
                "nested": {"config_id": 8},
                "prompt_id": 10
            })),
            is_favorite: false,
            created_at: dt(),
            updated_at: dt(),
        }
    }
}
