use super::super::super::ImportCanvasMediaInput;
use a4n_orchestrator::canvas::{
    canvas_audio_extension, required_canvas_audio_file_name, validate_canvas_audio_mime,
};
use base64::Engine as _;
use std::path::{Path, PathBuf};

const MAX_IMPORT_AUDIO_BYTES: u64 = 256 * 1024 * 1024;

enum CanvasAudioSourceData {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

pub(super) struct CanvasAudioImportSource {
    file_name: String,
    extension: &'static str,
    data: CanvasAudioSourceData,
}

impl CanvasAudioImportSource {
    pub(super) fn from_input(input: &ImportCanvasMediaInput) -> Result<Self, String> {
        if let Some(data_base64) = input.data_base64.as_deref() {
            return Self::from_upload(
                input
                    .file_name
                    .as_deref()
                    .ok_or_else(|| "导入画布音频缺少 file_name".to_string())?,
                input
                    .mime_type
                    .as_deref()
                    .unwrap_or("application/octet-stream"),
                data_base64,
            );
        }
        let file_path = input
            .file_path
            .as_deref()
            .ok_or_else(|| "导入画布音频缺少 file_path 或 data_base64".to_string())?;
        Self::from_path(file_path)
    }

    pub(super) fn file_name(&self) -> &str {
        &self.file_name
    }

    pub(super) fn extension(&self) -> &'static str {
        self.extension
    }

    pub(super) fn write_to(&self, output: &Path) -> Result<(), String> {
        match &self.data {
            CanvasAudioSourceData::Bytes(bytes) => {
                std::fs::write(output, bytes).map_err(|err| format!("写入画布音频文件失败：{err}"))
            }
            CanvasAudioSourceData::Path(path) => std::fs::copy(path, output)
                .map(|_| ())
                .map_err(|err| format!("复制画布音频文件失败：{err}")),
        }
    }

    fn from_upload(file_name: &str, mime_type: &str, data_base64: &str) -> Result<Self, String> {
        let file_name = required_canvas_audio_file_name(file_name)?;
        let extension = canvas_audio_extension(Path::new(&file_name))?;
        validate_canvas_audio_mime(mime_type, extension)?;
        let raw = data_base64
            .split_once(";base64,")
            .map(|(_, body)| body)
            .unwrap_or(data_base64)
            .trim();
        if raw.is_empty() {
            return Err("导入画布音频数据不能为空".to_string());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw)
            .map_err(|err| format!("导入画布音频 base64 解码失败：{err}"))?;
        if bytes.is_empty() {
            return Err("导入画布音频文件不能为空".to_string());
        }
        if bytes.len() as u64 > MAX_IMPORT_AUDIO_BYTES {
            return Err(format!(
                "导入画布音频不能超过 {}MB",
                MAX_IMPORT_AUDIO_BYTES / 1024 / 1024
            ));
        }
        Ok(Self {
            file_name,
            extension,
            data: CanvasAudioSourceData::Bytes(bytes),
        })
    }

    fn from_path(file_path: &str) -> Result<Self, String> {
        let trimmed = file_path.trim();
        if trimmed.is_empty() {
            return Err("请选择要导入的音频文件".to_string());
        }
        let path = PathBuf::from(trimmed);
        let metadata =
            std::fs::metadata(&path).map_err(|err| format!("无法读取导入音频文件：{err}"))?;
        if !metadata.is_file() {
            return Err("导入音频路径不是文件".to_string());
        }
        if metadata.len() > MAX_IMPORT_AUDIO_BYTES {
            return Err(format!(
                "导入画布音频不能超过 {}MB",
                MAX_IMPORT_AUDIO_BYTES / 1024 / 1024
            ));
        }
        let canonical = path
            .canonicalize()
            .map_err(|err| format!("无法解析导入音频路径：{err}"))?;
        let file_name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| "audio".to_string());
        let extension = canvas_audio_extension(&canonical)?;
        Ok(Self {
            file_name,
            extension,
            data: CanvasAudioSourceData::Path(canonical),
        })
    }
}
