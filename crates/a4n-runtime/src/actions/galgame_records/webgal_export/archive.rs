use super::contracts::WebgalExportAsset;
use super::rules::webgal_arg;
use crate::file_utils::canonicalize_data_file_path;
use a4n_orchestrator::galgame::GalgameScriptDetail;
use std::io::Write;
use std::path::Path;

pub(super) fn write_zip_text<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    path: &str,
    content: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    zip.start_file(path, options)
        .map_err(|err| format!("写入 WebGAL zip 条目失败: {path}: {err}"))?;
    zip.write_all(content.as_bytes())
        .map_err(|err| format!("写入 WebGAL zip 内容失败: {path}: {err}"))
}

pub(super) fn write_zip_data_file<W: Write + std::io::Seek>(
    data_dir: &Path,
    zip: &mut zip::ZipWriter<W>,
    asset: &WebgalExportAsset,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    let source = canonicalize_data_file_path(data_dir, &asset.data_path, "WebGAL 媒体文件")?;
    zip.start_file(&asset.zip_path, options)
        .map_err(|err| format!("写入 WebGAL 媒体条目失败: {}: {err}", asset.zip_path))?;
    let bytes = std::fs::read(&source)
        .map_err(|err| format!("读取 WebGAL 媒体文件失败: {}: {err}", source.display()))?;
    zip.write_all(&bytes)
        .map_err(|err| format!("写入 WebGAL 媒体文件失败: {}: {err}", asset.zip_path))
}

pub(super) fn webgal_config_text(detail: &GalgameScriptDetail) -> String {
    format!(
        "Game_name:{};\nGame_key:a4n_galgame_{};\n",
        webgal_arg(&detail.script.title),
        detail.script.id
    )
}
