pub(super) mod providers;
mod theme_system;

use super::types::{
    SettingsAllConfigImportCounts, SettingsAllConfigImportError, SettingsAllConfigImportInput,
    SettingsAllConfigRows,
};
use chrono::Utc;
use providers::{
    import_embedding_config_row, import_image_config_row, import_llm_config_row,
    import_video_config_row,
};
use sea_orm::{DatabaseConnection, TransactionTrait};
use theme_system::{import_system_setting_row, import_theme_row};

pub async fn import_settings_all_configs(
    db: &DatabaseConnection,
    input: SettingsAllConfigImportInput,
) -> Result<SettingsAllConfigImportCounts, SettingsAllConfigImportError> {
    let SettingsAllConfigImportInput { overwrite, rows } = input;
    let SettingsAllConfigRows {
        llm_configs,
        embedding_configs,
        image_configs,
        video_configs,
        themes,
        system_settings,
    } = rows;
    let txn = db
        .begin()
        .await
        .map_err(|source| SettingsAllConfigImportError::Database {
            operation: "开启 settings-all 配置导入事务失败",
            source,
        })?;
    let now = Utc::now().naive_utc();
    let mut counts = SettingsAllConfigImportCounts::default();

    for row in llm_configs {
        counts
            .llm
            .record(import_llm_config_row(&txn, row, overwrite, now).await?);
    }
    for row in embedding_configs {
        counts
            .embedding
            .record(import_embedding_config_row(&txn, row, overwrite, now).await?);
    }
    for row in image_configs {
        counts
            .image
            .record(import_image_config_row(&txn, row, overwrite, now).await?);
    }
    for row in video_configs {
        counts
            .video
            .record(import_video_config_row(&txn, row, overwrite, now).await?);
    }
    for row in themes {
        counts
            .theme
            .record(import_theme_row(&txn, row, overwrite, now).await?);
    }
    for row in system_settings {
        counts
            .system_settings
            .record(import_system_setting_row(&txn, row, overwrite, now).await?);
    }

    txn.commit()
        .await
        .map_err(|source| SettingsAllConfigImportError::Database {
            operation: "提交 settings-all 配置导入事务失败",
            source,
        })?;
    Ok(counts)
}
