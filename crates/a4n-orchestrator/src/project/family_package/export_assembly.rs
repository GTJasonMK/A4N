//! 项目族导出：选择校验、快照覆盖校验与 manifest 组装（不含磁盘媒体读取）。

use std::collections::HashSet;

use a4n_core::models::project::{novel_project, project_source_snapshot};
use a4n_db::project::ProjectFamilyExportRows;
use chrono::Utc;

use super::{MediaFileEntry, PROJECT_FAMILY_PACKAGE_VERSION, ProjectFamilyPackage};
use crate::project::required_path_extension;

/// 校验根项目与派生项目归属同一用户且挂在正确 root 下。
pub fn validate_export_same_user(
    root: &novel_project::Model,
    derived_projects: &[novel_project::Model],
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
pub fn validate_export_snapshot_coverage(
    expected_ids: &[i64],
    snapshots: &[project_source_snapshot::Model],
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
pub fn assemble_project_family_export_package(
    root: novel_project::Model,
    derived_projects: Vec<novel_project::Model>,
    mode: &str,
    export_rows: ProjectFamilyExportRows,
    media_files: Vec<MediaFileEntry>,
) -> Result<ProjectFamilyPackage, String> {
    validate_export_same_user(&root, &derived_projects)?;
    validate_export_snapshot_coverage(&export_rows.snapshot_ids, &export_rows.source_snapshots)?;

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

/// 导出媒体包内相对路径：`media/{id}/{field}.{ext}`。
pub fn family_media_package_path(
    media_asset_id: i64,
    field: &str,
    original_path: &str,
) -> Result<String, String> {
    let extension = required_path_extension(original_path, "媒体文件")?;
    Ok(format!("media/{media_asset_id}/{field}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::project::novel_project;
    use chrono::NaiveDate;

    fn dt() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn project(id: i64, user_id: i64, root: Option<i64>) -> novel_project::Model {
        novel_project::Model {
            id,
            user_id,
            title: format!("p{id}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
            project_kind: "text".to_string(),
            source_mode: if root.is_some() {
                "derived".to_string()
            } else {
                "self".to_string()
            },
            root_project_id: root,
            source_snapshot_id: None,
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
        let err = family_media_package_path(1, "file", "noext").unwrap_err();
        assert!(err.contains("扩展名"));
        assert_eq!(
            family_media_package_path(3, "thumbnail", "a/b.png").unwrap(),
            "media/3/thumbnail.png"
        );
    }
}
