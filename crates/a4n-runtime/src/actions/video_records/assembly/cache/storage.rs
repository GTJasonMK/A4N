use super::super::contracts::AssemblyCacheManifest;
use crate::actions::video_generation::media::remove_partial_video_file;
use crate::file_utils::remove_created_file_on_error_sync;
use std::path::Path;

use super::expiry::assembly_cache_is_fresh;

pub(in crate::actions::video_records) fn assembly_cache_matches(
    output_path: &Path,
    manifest_path: &Path,
    expected: &AssemblyCacheManifest,
) -> Result<bool, String> {
    if !output_path.is_file()
        || !manifest_path.is_file()
        || !assembly_cache_is_fresh(manifest_path)?
    {
        return Ok(false);
    }
    let bytes =
        std::fs::read(manifest_path).map_err(|e| format!("无法读取成片预览缓存清单：{e}"))?;
    let actual: AssemblyCacheManifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("成片预览缓存清单格式无效：{e}"))?;
    Ok(&actual == expected)
}

pub(in crate::actions::video_records) fn write_assembly_cache_manifest(
    manifest_path: &Path,
    manifest: &AssemblyCacheManifest,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("无法序列化成片预览缓存清单：{e}"))?;
    let tmp_path = manifest_path.with_extension("tmp");
    remove_created_file_on_error_sync(&tmp_path, "成片预览缓存临时清单", || {
        std::fs::write(&tmp_path, bytes).map_err(|e| format!("无法写入成片预览缓存清单：{e}"))?;
        remove_partial_video_file(manifest_path, "旧成片预览缓存清单")?;
        std::fs::rename(&tmp_path, manifest_path)
            .map_err(|e| format!("无法保存成片预览缓存清单：{e}"))
    })
}
