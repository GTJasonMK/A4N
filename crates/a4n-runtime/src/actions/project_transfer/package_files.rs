use super::ExportProjectPackageData;
#[cfg(test)]
use super::PROJECT_FAMILY_PACKAGE_SUFFIX;
use a4n_core::models::project::novel_project;
use base64::Engine as _;

pub(super) fn export_package_data(
    project: &novel_project::Model,
    bytes: Vec<u8>,
    file_suffix: &str,
) -> ExportProjectPackageData {
    ExportProjectPackageData {
        file_name: project_package_file_name(project, file_suffix),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    }
}

pub(super) fn validate_uploaded_project_file_name(
    file_name: &str,
    expected_suffix: &str,
    label: &str,
) -> Result<(), String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}文件名不能为空"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(format!("{label}文件名不能包含路径"));
    }
    if !trimmed.to_ascii_lowercase().ends_with(expected_suffix) {
        return Err(format!("{label}文件名必须以 {expected_suffix} 结尾"));
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

fn project_package_file_name(project: &novel_project::Model, file_suffix: &str) -> String {
    let stem = sanitize_file_name_component(&project.title);
    let stem = if stem.is_empty() {
        format!("project-{}", project.id)
    } else {
        stem
    };
    format!("{stem}{file_suffix}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_file_name_requires_exact_package_suffix() {
        validate_uploaded_project_file_name(
            "novel.a4n-family.zip",
            PROJECT_FAMILY_PACKAGE_SUFFIX,
            "项目族包",
        )
        .expect("项目族包后缀应通过");
        assert!(
            validate_uploaded_project_file_name(
                "novel.a4n-project.zip",
                PROJECT_FAMILY_PACKAGE_SUFFIX,
                "项目族包",
            )
            .is_err()
        );
        assert!(
            validate_uploaded_project_file_name(
                "novel.a4n-family.zip/other.zip",
                PROJECT_FAMILY_PACKAGE_SUFFIX,
                "项目族包",
            )
            .is_err()
        );
        assert!(
            validate_uploaded_project_file_name(
                "folder\\novel.a4n-family.zip",
                PROJECT_FAMILY_PACKAGE_SUFFIX,
                "项目族包",
            )
            .is_err()
        );
    }
}
