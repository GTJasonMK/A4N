use super::super::types::{SettingsAllConfigImportError, SettingsAllConfigImportInput};
use super::contracts::{DefaultImportPolicy, DefaultStateRow};
use super::policy::ensure_single_default_after_import;
use a4n_db_models::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};
use sea_orm::{EntityTrait, QuerySelect};

pub async fn validate_settings_all_config_import_defaults(
    db: &impl crate::DbExecutor,
    input: &SettingsAllConfigImportInput,
) -> Result<(), SettingsAllConfigImportError> {
    let existing_llm = llm_config::Entity::find()
        .select_only()
        .column(llm_config::Column::Name)
        .column(llm_config::Column::IsDefault)
        .into_tuple::<(String, bool)>()
        .all(db)
        .await
        .map_err(|source| SettingsAllConfigImportError::Database {
            operation: "读取现有 LLM 配置失败",
            source,
        })?
        .into_iter()
        .map(|(name, is_default)| DefaultStateRow { name, is_default })
        .collect::<Vec<_>>();
    ensure_single_default_after_import(
        "默认 LLM 配置",
        &existing_llm,
        &input.rows.llm_configs,
        input.overwrite,
        DefaultImportPolicy::PreserveIfConfigured,
        |row| row.name.as_str(),
        |row| row.is_default,
    )?;

    let existing_embedding = embedding_config::Entity::find()
        .select_only()
        .column(embedding_config::Column::Name)
        .column(embedding_config::Column::IsDefault)
        .into_tuple::<(String, bool)>()
        .all(db)
        .await
        .map_err(|source| SettingsAllConfigImportError::Database {
            operation: "读取现有 Embedding 配置失败",
            source,
        })?
        .into_iter()
        .map(|(name, is_default)| DefaultStateRow { name, is_default })
        .collect::<Vec<_>>();
    ensure_single_default_after_import(
        "默认 Embedding 配置",
        &existing_embedding,
        &input.rows.embedding_configs,
        input.overwrite,
        DefaultImportPolicy::Required,
        |row| row.name.as_str(),
        |row| row.is_default,
    )?;

    let existing_image = image_gen_config::Entity::find()
        .select_only()
        .column(image_gen_config::Column::Name)
        .column(image_gen_config::Column::IsDefault)
        .into_tuple::<(String, bool)>()
        .all(db)
        .await
        .map_err(|source| SettingsAllConfigImportError::Database {
            operation: "读取现有图片配置失败",
            source,
        })?
        .into_iter()
        .map(|(name, is_default)| DefaultStateRow { name, is_default })
        .collect::<Vec<_>>();
    ensure_single_default_after_import(
        "默认图片配置",
        &existing_image,
        &input.rows.image_configs,
        input.overwrite,
        DefaultImportPolicy::PreserveIfConfigured,
        |row| row.name.as_str(),
        |row| row.is_default,
    )?;

    let existing_video = video_gen_config::Entity::find()
        .select_only()
        .column(video_gen_config::Column::Name)
        .column(video_gen_config::Column::IsDefault)
        .into_tuple::<(String, bool)>()
        .all(db)
        .await
        .map_err(|source| SettingsAllConfigImportError::Database {
            operation: "读取现有视频配置失败",
            source,
        })?
        .into_iter()
        .map(|(name, is_default)| DefaultStateRow { name, is_default })
        .collect::<Vec<_>>();
    ensure_single_default_after_import(
        "默认视频配置",
        &existing_video,
        &input.rows.video_configs,
        input.overwrite,
        DefaultImportPolicy::PreserveIfConfigured,
        |row| row.name.as_str(),
        |row| row.is_default,
    )
}
