#[cfg(test)]
use super::PROJECT_FAMILY_PACKAGE_SUFFIX;
use super::{ExportProjectPackageData, ImportProjectPackageUploadInput};
use a4n_core::models::project::novel_project;
use a4n_orchestrator::project::ImportProjectPackageApplicationInput;
use base64::Engine as _;
use std::path::Path;

pub(super) fn export_package_data(
    project: &novel_project::Model,
    bytes: Vec<u8>,
    file_suffix: &str,
) -> Result<ExportProjectPackageData, String> {
    Ok(ExportProjectPackageData {
        file_name: project_package_file_name(&project.title, file_suffix)?,
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

pub(super) fn project_package_import_input(
    user_id: i64,
    input: ImportProjectPackageUploadInput,
    expected_suffix: &str,
    label: &str,
) -> Result<ImportProjectPackageApplicationInput, String> {
    validate_uploaded_project_file_name(&input.file_name, expected_suffix, label)?;
    Ok(ImportProjectPackageApplicationInput {
        user_id,
        package_bytes: decode_uploaded_project_file(&input.data_base64, label)?,
    })
}

pub(super) fn project_package_media_zip_entries<'a>(
    data_dir: &Path,
    media_files: impl IntoIterator<Item = (&'a str, &'a str)>,
    source_label: &str,
) -> Result<Vec<a4n_export_core::ExportZipEntryBytes>, String> {
    let mut entries = Vec::new();
    for (original_path, package_path) in media_files {
        let source =
            crate::file_utils::canonicalize_data_file_path(data_dir, original_path, source_label)?;
        let bytes = std::fs::read(&source)
            .map_err(|err| format!("读取媒体文件失败: {}: {err}", source.display()))?;
        entries.push(a4n_export_core::ExportZipEntryBytes {
            path: package_path.to_string(),
            bytes,
            label: format!("媒体文件 {package_path}"),
        });
    }
    Ok(entries)
}

fn validate_uploaded_project_file_name(
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

fn decode_uploaded_project_file(data_base64: &str, label: &str) -> Result<Vec<u8>, String> {
    let raw = data_base64.trim();
    if raw.is_empty() {
        return Err(format!("{label}内容不能为空"));
    }
    base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|err| format!("{label} base64 解码失败：{err}"))
}

fn project_package_file_name(title: &str, file_suffix: &str) -> Result<String, String> {
    let stem = sanitize_file_name_component(title);
    if stem.is_empty() {
        return Err("项目标题无法生成导出文件名".to_string());
    }
    Ok(format!("{stem}{file_suffix}"))
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

    #[test]
    fn upload_content_requires_raw_base64() {
        assert_eq!(
            decode_uploaded_project_file("aGVsbG8=", "项目族包").expect("原始 Base64 应通过"),
            b"hello"
        );
        assert!(
            decode_uploaded_project_file("data:application/zip;base64,aGVsbG8=", "项目族包")
                .is_err()
        );
    }

    #[test]
    fn export_file_name_rejects_empty_sanitized_title() {
        assert!(project_package_file_name("...", PROJECT_FAMILY_PACKAGE_SUFFIX).is_err());
    }
}
