use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

pub(super) fn normalize_relative_data_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => out.push(value),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return Err(format!("{label}不在应用数据目录内"));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("{label}路径必须是应用数据目录内的相对路径"));
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err(format!("{label}路径不能为空"));
    }
    Ok(out)
}

pub(super) fn validate_missing_delete_target_parent(
    canonical_data_dir: &Path,
    candidate: &Path,
    label: &str,
) -> Result<(), String> {
    for ancestor in candidate.ancestors().skip(1) {
        match std::fs::metadata(ancestor) {
            Ok(metadata) => {
                if !metadata.is_dir() {
                    return Err(format!("{label}父路径不是目录"));
                }
                let canonical_ancestor = ancestor
                    .canonicalize()
                    .map_err(|err| format!("无法解析{label}父路径：{err}"))?;
                if !canonical_ancestor.starts_with(canonical_data_dir) {
                    return Err(format!("{label}不在应用数据目录内"));
                }
                return Ok(());
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(format!("无法读取{label}父路径：{err}")),
        }
    }
    Err(format!("{label}不在应用数据目录内"))
}
