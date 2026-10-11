use crate::file_utils::promote_data_file;
use a4n_image_host_contract::{
    PersistGeneratedImageFileInput, PersistedGeneratedImageFile, PrepareGeneratedImageFileInput,
    PreparedGeneratedImageFile,
};
use a4n_storage::paths::ProjectPaths;
use a4n_storage::{data_file_record_facts, write_new_synced_data_file};
use std::path::{Component, Path, PathBuf};

pub(crate) fn prepare_generated_image_file(
    data_dir: &Path,
    input: PrepareGeneratedImageFileInput<'_>,
) -> Result<PreparedGeneratedImageFile, String> {
    let kind = required_filename_segment(input.image_type, "kind")?;
    let format = required_filename_segment(input.format, "format")?;
    let publication_id = required_filename_segment(input.publication_id, "publication_id")?;
    let suffix = match input.file_extra {
        Some(value) => format!("_{}", required_filename_segment(value, "extra")?),
        None => String::new(),
    };
    let dir = image_output_dir(
        data_dir,
        input.project_id,
        input.image_type,
        input.chapter_id,
    )?;
    std::fs::create_dir_all(&dir).map_err(|error| format!("无法创建图片目录 {dir:?}：{error}"))?;
    let stem = format!("{kind}{suffix}_{publication_id}");
    let staging = dir.join(format!("{stem}.{format}.part"));
    let final_path = dir.join(format!("{stem}.{format}"));
    Ok(PreparedGeneratedImageFile {
        staging_path: a4n_storage::paths::data_dir_relative_path(
            data_dir,
            &staging,
            input.file_context,
        )?,
        final_path: a4n_storage::paths::data_dir_relative_path(
            data_dir,
            &final_path,
            input.file_context,
        )?,
    })
}

pub(crate) fn persist_reserved_generated_image_file(
    data_dir: &Path,
    input: PersistGeneratedImageFileInput<'_>,
    prepared: &PreparedGeneratedImageFile,
) -> Result<PersistedGeneratedImageFile, String> {
    resolve_reserved_path(data_dir, &prepared.staging_path, input.file_context)?;
    let final_path = resolve_reserved_path(data_dir, &prepared.final_path, input.file_context)?;
    write_new_synced_data_file(
        data_dir,
        &prepared.staging_path,
        input.bytes,
        input.file_context,
    )?;
    promote_data_file(
        data_dir,
        &prepared.staging_path,
        &prepared.final_path,
        input.file_context,
    )?;
    let (relative_path, file_size) = data_file_record_facts(
        data_dir,
        &final_path,
        input.file_context,
        input.file_context,
    )?;
    Ok(PersistedGeneratedImageFile {
        relative_path,
        file_size,
    })
}

fn resolve_reserved_path(
    data_dir: &Path,
    relative_path: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let raw = Path::new(relative_path.trim());
    if raw.is_absolute()
        || raw.as_os_str().is_empty()
        || raw
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("{label}暂存/最终路径无效"));
    }
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("无法解析数据目录：{error}"))?;
    let file_name = raw
        .file_name()
        .ok_or_else(|| format!("{label}路径缺少文件名"))?;
    let parent = canonical_data_dir
        .join(raw)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("{label}路径缺少父目录"))?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| format!("无法解析{label}目录：{error}"))?;
    if !canonical_parent.starts_with(&canonical_data_dir) {
        return Err(format!("{label}路径不在应用数据目录内"));
    }
    Ok(canonical_parent.join(file_name))
}

fn image_output_dir(
    data_dir: &Path,
    project_id: i64,
    image_type: &str,
    chapter_id: Option<i64>,
) -> Result<PathBuf, String> {
    let project_paths = ProjectPaths::new(data_dir, project_id);
    match (image_type, chapter_id) {
        ("portrait", None) => Ok(project_paths.portraits_dir()),
        (_, Some(chapter_id)) if chapter_id > 0 => Ok(project_paths.chapter_images_dir(chapter_id)),
        (_, Some(chapter_id)) => Err(format!(
            "章节图片目录需要有效 chapter_id，收到 {chapter_id}"
        )),
        (_, None) => Ok(project_paths.images_dir()),
    }
}

pub(super) fn required_filename_segment<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("图片文件名片段 {field} 不能为空"));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(format!(
            "图片文件名片段 {field} 只能包含 ASCII 字母、数字、下划线或短横线"
        ));
    }
    Ok(value)
}
