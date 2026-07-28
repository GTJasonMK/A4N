//! 项目 Host 输入与复制/重置路径的纯校验规则（无 DB、无运行时依赖）。

use a4n_core::models::project::novel_project;
use a4n_core::state::validator::StateValidator;

use super::{PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO, SOURCE_MODE_FREEFORM};

/// 复制项目时的落库路径选择（不含权限与文件副作用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectDuplicatePath {
    /// 非 freeform 文本项目：纯文本复制。
    Text,
    /// 非 freeform 派生项目：含媒体复制。
    Derived,
    /// freeform 项目：自由画布复制。
    Freeform,
}

/// 一页制分页：page 从 1 起，计算 offset。
pub fn project_list_offset(page: u64, limit: u64) -> Result<u64, String> {
    page.checked_sub(1)
        .and_then(|page_index| page_index.checked_mul(limit))
        .ok_or_else(|| "项目分页参数过大".to_string())
}

/// 必填文本：trim 后非空。
pub fn required_text<'a>(value: &'a str, field: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{field} 不能为空"))
    } else {
        Ok(value)
    }
}

/// 可选字段作为必填：Some 且 trim 后非空。
pub fn required_optional_text<'a>(value: Option<&'a str>, field: &str) -> Result<&'a str, String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{field} 不能为空"))
}

/// 可选文本：None 或空白视为空串。
pub fn optional_text(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}

/// 解析并规范化项目状态字符串（用于分支创建等）。
pub fn required_project_status(value: &str, field: &str) -> Result<String, String> {
    let value = required_text(value, field)?;
    let status = StateValidator::parse_status(value).map_err(|e| e.to_string())?;
    Ok(status.as_str().to_string())
}

/// 复制/分支操作仅允许文本项目。
pub fn ensure_text_project_for_copy(
    project: &novel_project::Model,
    operation: &str,
) -> Result<(), String> {
    if project.project_kind != PROJECT_KIND_TEXT {
        Err(format!("{operation}仅支持文本项目"))
    } else {
        Ok(())
    }
}

/// 按 source_mode / project_kind 选择 duplicate 落库路径。
pub fn resolve_project_duplicate_path(project: &novel_project::Model) -> ProjectDuplicatePath {
    if project.source_mode != SOURCE_MODE_FREEFORM && project.project_kind == PROJECT_KIND_TEXT {
        ProjectDuplicatePath::Text
    } else if project.source_mode != SOURCE_MODE_FREEFORM {
        ProjectDuplicatePath::Derived
    } else {
        ProjectDuplicatePath::Freeform
    }
}

/// 是否走「自由画布视频生成数据重置」专用入口。
pub fn is_freeform_video_generated_data_reset_project(project: &novel_project::Model) -> bool {
    project.project_kind == PROJECT_KIND_VIDEO && project.source_mode == SOURCE_MODE_FREEFORM
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(kind: &str, source_mode: &str) -> novel_project::Model {
        let ts = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        novel_project::Model {
            id: 1,
            user_id: 1,
            title: "t".into(),
            description: String::new(),
            project_type: "novel_long".into(),
            project_kind: kind.into(),
            source_mode: source_mode.into(),
            root_project_id: None,
            parent_project_id: None,
            source_snapshot_id: None,
            source_sync_status: "current".into(),
            derived_project_status: None,
            status: "draft".into(),
            lifecycle_gate: "active".into(),
            target_chapters: 0,
            current_chapter: 0,
            total_chapters: None,
            chapters_per_part: None,
            avatar_svg: None,
            avatar_animal: None,
            raw_blueprint: None,
            import_analysis_progress: None,
            export_config: None,
            created_at: ts,
            updated_at: ts,
        }
    }

    #[test]
    fn project_list_offset_uses_one_based_pages() {
        assert_eq!(project_list_offset(1, 20), Ok(0));
        assert_eq!(project_list_offset(3, 20), Ok(40));
    }

    #[test]
    fn project_list_offset_rejects_overflow() {
        let err = project_list_offset(u64::MAX, u64::MAX).expect_err("overflow");
        assert!(err.contains("分页参数过大"));
    }

    #[test]
    fn required_and_optional_text_trim() {
        assert_eq!(required_text(" 标题 ", "项目标题").unwrap(), "标题");
        assert!(required_text("  ", "项目标题").is_err());
        assert_eq!(
            required_optional_text(Some(" novel_long "), "项目类型").unwrap(),
            "novel_long"
        );
        assert!(required_optional_text(None, "项目类型").is_err());
        assert_eq!(optional_text(None), "");
        assert_eq!(optional_text(Some(" 简介 ")), "简介");
    }

    #[test]
    fn ensure_text_project_for_copy_rejects_non_text() {
        assert!(ensure_text_project_for_copy(&project("text", "root"), "创建项目分支").is_ok());
        assert!(
            ensure_text_project_for_copy(&project("video", "root"), "创建项目分支")
                .unwrap_err()
                .contains("仅支持文本项目")
        );
    }

    #[test]
    fn resolve_project_duplicate_path_covers_modes() {
        assert_eq!(
            resolve_project_duplicate_path(&project("text", "root")),
            ProjectDuplicatePath::Text
        );
        assert_eq!(
            resolve_project_duplicate_path(&project("galgame", "root")),
            ProjectDuplicatePath::Derived
        );
        assert_eq!(
            resolve_project_duplicate_path(&project("video", SOURCE_MODE_FREEFORM)),
            ProjectDuplicatePath::Freeform
        );
    }

    #[test]
    fn freeform_video_reset_path_only_for_video_freeform() {
        assert!(is_freeform_video_generated_data_reset_project(&project(
            "video",
            SOURCE_MODE_FREEFORM
        )));
        assert!(!is_freeform_video_generated_data_reset_project(&project(
            "text",
            SOURCE_MODE_FREEFORM
        )));
        assert!(!is_freeform_video_generated_data_reset_project(&project(
            "video", "root"
        )));
    }
}
