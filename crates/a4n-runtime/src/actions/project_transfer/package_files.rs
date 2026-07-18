use super::contracts::ExportProjectPackageData;
use crate::state::AppState;
use a4n_core::models::project::novel_project;
use base64::Engine as _;
use std::path::Path;

pub(super) fn export_package_data(
    project: &novel_project::Model,
    bytes: Vec<u8>,
) -> ExportProjectPackageData {
    ExportProjectPackageData {
        file_name: project_package_file_name(project),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    }
}

pub(super) async fn project_record_by_id(
    state: &AppState,
    project_id: i64,
) -> Result<novel_project::Model, String> {
    state
        .orchestrator
        .project_by_id(project_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
}

pub(super) fn validate_uploaded_project_file_name(
    file_name: &str,
    expected_extension: &str,
    label: &str,
) -> Result<(), String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}文件名不能为空"));
    }
    let name = Path::new(trimmed)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label}文件名无效"))?;
    if name != trimmed {
        return Err(format!("{label}文件名不能包含路径"));
    }
    let extension = Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .ok_or_else(|| format!("{label}必须是 .{expected_extension} 文件"))?;
    if !extension.eq_ignore_ascii_case(expected_extension) {
        return Err(format!("{label}必须是 .{expected_extension} 文件"));
    }
    Ok(())
}

pub(super) fn decode_uploaded_project_file(
    data_base64: &str,
    label: &str,
) -> Result<Vec<u8>, String> {
    let raw = data_base64
        .split_once(";base64,")
        .map(|(_, body)| body)
        .unwrap_or(data_base64)
        .trim();
    if raw.is_empty() {
        return Err(format!("{label}内容不能为空"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|err| format!("{label} base64 解码失败：{err}"))?;
    if bytes.is_empty() {
        return Err(format!("{label}内容不能为空"));
    }
    Ok(bytes)
}

fn project_package_file_name(project: &novel_project::Model) -> String {
    let stem = sanitize_file_name_component(&project.title);
    let stem = if stem.is_empty() {
        format!("project-{}", project.id)
    } else {
        stem
    };
    format!("{stem}.a4n-project.zip")
}

fn sanitize_file_name_component(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string()
}
