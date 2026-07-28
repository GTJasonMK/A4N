use std::path::Path;

use anyhow::{Context, anyhow};
use serde_json::{Value, json};

pub fn ensure_data_directory_manifest(
    root: &Path,
    owner: &str,
    classification: &str,
    format_version: u32,
    backup_required: bool,
    recovery: &str,
    details: Value,
) -> anyhow::Result<()> {
    let (manifest_file, expected) = expected_manifest(
        owner,
        classification,
        format_version,
        backup_required,
        recovery,
        details,
    )?;
    std::fs::create_dir_all(root)
        .with_context(|| format!("无法创建 sidecar 数据目录 {}", root.display()))?;
    let path = root.join(&manifest_file);
    if path.exists() {
        return require_expected_manifest(&path, &expected);
    }

    let temp_path = root.join(format!("{manifest_file}.tmp"));
    std::fs::write(&temp_path, serde_json::to_vec_pretty(&expected)?)
        .with_context(|| format!("无法写入 sidecar 数据 manifest {}", temp_path.display()))?;
    std::fs::rename(&temp_path, &path)
        .with_context(|| format!("无法提交 sidecar 数据 manifest {}", path.display()))?;
    Ok(())
}

pub fn require_data_directory_manifest(
    root: &Path,
    owner: &str,
    classification: &str,
    format_version: u32,
    backup_required: bool,
    recovery: &str,
    details: Value,
) -> anyhow::Result<()> {
    let (manifest_file, expected) = expected_manifest(
        owner,
        classification,
        format_version,
        backup_required,
        recovery,
        details,
    )?;
    let path = root.join(manifest_file);
    if !path.is_file() {
        return Err(anyhow!("sidecar 数据 manifest 缺失 {}", path.display()));
    }
    require_expected_manifest(&path, &expected)
}

fn expected_manifest(
    owner: &str,
    classification: &str,
    format_version: u32,
    backup_required: bool,
    recovery: &str,
    details: Value,
) -> anyhow::Result<(String, Value)> {
    if owner.is_empty()
        || !owner
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(anyhow!("sidecar 数据 manifest owner 非法: {owner:?}"));
    }
    Ok((
        format!(".a4n-{owner}-data.json"),
        json!({
            "owner": owner,
            "classification": classification,
            "format_version": format_version,
            "backup_required": backup_required,
            "recovery": recovery,
            "details": details,
        }),
    ))
}

fn require_expected_manifest(path: &Path, expected: &Value) -> anyhow::Result<()> {
    let actual: Value = serde_json::from_slice(
        &std::fs::read(path)
            .with_context(|| format!("无法读取 sidecar 数据 manifest {}", path.display()))?,
    )
    .with_context(|| format!("sidecar 数据 manifest 已损坏 {}", path.display()))?;
    if actual != *expected {
        return Err(anyhow!(
            "sidecar 数据 manifest 不兼容 {}：expected={}, actual={}",
            path.display(),
            expected,
            actual
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_manifest_is_idempotent_and_rejects_incompatible_versions() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "a4n-sidecar-manifest-{}-{nonce}",
            std::process::id()
        ));
        let details = json!({"engine": "test"});

        ensure_data_directory_manifest(
            &root,
            "test-owner",
            "rebuildable_cache",
            1,
            false,
            "delete_and_rebuild",
            details.clone(),
        )
        .unwrap();
        require_data_directory_manifest(
            &root,
            "test-owner",
            "rebuildable_cache",
            1,
            false,
            "delete_and_rebuild",
            details.clone(),
        )
        .unwrap();
        ensure_data_directory_manifest(
            &root,
            "test-owner",
            "rebuildable_cache",
            1,
            false,
            "delete_and_rebuild",
            details.clone(),
        )
        .unwrap();
        let error = ensure_data_directory_manifest(
            &root,
            "test-owner",
            "rebuildable_cache",
            2,
            false,
            "delete_and_rebuild",
            details,
        )
        .expect_err("incompatible data format must fail closed");
        assert!(error.to_string().contains("manifest 不兼容"));

        std::fs::remove_file(root.join(".a4n-test-owner-data.json")).unwrap();
        let missing = require_data_directory_manifest(
            &root,
            "test-owner",
            "rebuildable_cache",
            1,
            false,
            "delete_and_rebuild",
            json!({"engine": "test"}),
        )
        .expect_err("required manifest must not be created implicitly");
        assert!(missing.to_string().contains("manifest 缺失"));

        std::fs::remove_dir_all(root).unwrap();
    }
}
