use super::super::types::ImportAllResult;
use a4n_core::error::AppError;
use a4n_orchestrator::config::{SettingsAllConfigImportCounts, SettingsAllImportTableCounts};

pub(super) fn import_result_from_counts(
    counts: SettingsAllConfigImportCounts,
    prompt_imported: usize,
) -> ImportAllResult {
    ImportAllResult {
        success: true,
        message: "全量导入完成".to_string(),
        details: vec![
            import_detail("LLM 配置", counts.llm),
            import_detail("嵌入配置", counts.embedding),
            import_detail("图片配置", counts.image),
            import_detail("视频配置", counts.video),
            format!("提示词：导入 {prompt_imported}"),
            import_detail("主题", counts.theme),
            import_detail("系统设置", counts.system_settings),
        ],
    }
}

fn import_detail(label: &str, counts: SettingsAllImportTableCounts) -> String {
    format!(
        "{}：导入 {}，跳过 {}",
        label, counts.imported, counts.skipped
    )
}

pub(super) fn config_import_error(error: AppError) -> String {
    error.to_string()
}
