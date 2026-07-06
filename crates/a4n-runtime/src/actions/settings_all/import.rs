use super::types::{
    AllConfigsExport, ImageRow, ImportAllConfigsInput, ImportAllResult, PromptRow, ThemeRow,
    VideoRow,
};
use super::validation::{normalize_all_configs_export, validate_all_configs_export};
use crate::AppState;
use a4n_core::config::app_config::LlmMaxTokensConfig;
use a4n_core::error::AppError;
use a4n_core::models::ai::{
    embedding_config, image_gen_config, llm_config, system_settings, video_gen_config,
};
use a4n_core::models::user::theme_config;
use a4n_engine::prompt::engine::PromptEngine;
use chrono::Utc;
use sea_orm::*;
use tera::Tera;

macro_rules! import_counted_rows {
    ($details:ident, $label:literal, $rows:expr, $import_row:ident, $db:expr, $overwrite:expr, $now:expr) => {{
        let mut imported = 0usize;
        let mut skipped = 0usize;
        for row in $rows {
            if $import_row($db, row, $overwrite, $now).await? {
                imported += 1;
            } else {
                skipped += 1;
            }
        }
        $details.push(format!("{}：导入 {}，跳过 {}", $label, imported, skipped));
    }};
}

pub async fn import_all_configs(
    state: &AppState,
    input: ImportAllConfigsInput,
) -> Result<ImportAllResult, AppError> {
    import_all_configs_inner(state, input)
        .await
        .map_err(AppError::InvalidArgument)
}

async fn import_all_configs_inner(
    state: &AppState,
    input: ImportAllConfigsInput,
) -> Result<ImportAllResult, String> {
    let mut incoming: AllConfigsExport =
        serde_json::from_str(&input.settings_json).map_err(|e| format!("JSON 解析失败：{e}"))?;
    validate_all_configs_export(&incoming)?;
    normalize_all_configs_export(&mut incoming)?;
    validate_prompt_import_targets(&incoming, state.engine.as_ref())?;

    let db = state.db.as_ref();
    let overwrite = input.overwrite;
    validate_config_default_import_results(db, &incoming, overwrite).await?;
    let prompt_rows = std::mem::take(&mut incoming.prompts_modified);
    let prompt_snapshots = prompt_import_snapshots(state.engine.as_ref(), &prompt_rows)?;
    let prompt_imported = import_prompt_rows(state.engine.as_ref(), prompt_rows).await?;

    let db_import_result: Result<ImportAllResult, String> = async {
        let txn = db.begin().await.map_err(|e| e.to_string())?;
        let now = Utc::now().naive_utc();
        let mut details: Vec<String> = Vec::new();

        {
            let rows = incoming.llm_configs;
            let mut imported = 0usize;
            let mut skipped = 0usize;
            for row in rows {
                let existing = llm_config::Entity::find()
                    .filter(llm_config::Column::Name.eq(&row.name))
                    .one(&txn)
                    .await
                    .map_err(|e| e.to_string())?;
                match (existing, overwrite) {
                    (Some(_), false) => skipped += 1,
                    (Some(e), true) => {
                        if row.is_default {
                            clear_llm_defaults(&txn).await?;
                        }
                        let mut a: llm_config::ActiveModel = e.into();
                        a.provider = Set(row.provider);
                        a.base_url = Set(row.base_url);
                        a.api_key_encrypted = Set(row.api_key_encrypted);
                        a.model = Set(row.model);
                        a.max_concurrent = Set(row.max_concurrent);
                        a.timeout_secs = Set(row.timeout_secs);
                        a.max_context_tokens = Set(row.max_context_tokens);
                        a.max_output_tokens = Set(row.max_output_tokens);
                        a.request_options_json = Set(row.request_options_json);
                        a.is_default = Set(row.is_default);
                        a.updated_at = Set(now);
                        a.update(&txn).await.map_err(|e| e.to_string())?;
                        imported += 1;
                    }
                    (None, _) => {
                        if row.is_default {
                            clear_llm_defaults(&txn).await?;
                        }
                        llm_config::ActiveModel {
                            name: Set(row.name),
                            provider: Set(row.provider),
                            base_url: Set(row.base_url),
                            api_key_encrypted: Set(row.api_key_encrypted),
                            model: Set(row.model),
                            max_concurrent: Set(row.max_concurrent),
                            timeout_secs: Set(row.timeout_secs),
                            max_context_tokens: Set(row.max_context_tokens),
                            max_output_tokens: Set(row.max_output_tokens),
                            request_options_json: Set(row.request_options_json),
                            is_default: Set(row.is_default),
                            created_at: Set(now),
                            updated_at: Set(now),
                            ..Default::default()
                        }
                        .insert(&txn)
                        .await
                        .map_err(|e| e.to_string())?;
                        imported += 1;
                    }
                }
            }
            details.push(format!("LLM 配置：导入 {imported}，跳过 {skipped}"));
        }

        {
            let rows = incoming.embedding_configs;
            let mut imported = 0usize;
            let mut skipped = 0usize;
            for row in rows {
                let existing = embedding_config::Entity::find()
                    .filter(embedding_config::Column::Name.eq(&row.name))
                    .one(&txn)
                    .await
                    .map_err(|e| e.to_string())?;
                match (existing, overwrite) {
                    (Some(_), false) => skipped += 1,
                    (Some(e), true) => {
                        if row.is_default {
                            clear_embedding_defaults(&txn).await?;
                        }
                        let mut a: embedding_config::ActiveModel = e.into();
                        a.backend = Set(row.backend);
                        a.base_url = Set(row.base_url);
                        a.api_key_encrypted = Set(row.api_key_encrypted);
                        a.model = Set(row.model);
                        a.dimension = Set(row.dimension);
                        a.batch_size = Set(row.batch_size);
                        a.local_model_path = Set(row.local_model_path);
                        a.is_default = Set(row.is_default);
                        a.updated_at = Set(now);
                        a.update(&txn).await.map_err(|e| e.to_string())?;
                        imported += 1;
                    }
                    (None, _) => {
                        if row.is_default {
                            clear_embedding_defaults(&txn).await?;
                        }
                        embedding_config::ActiveModel {
                            name: Set(row.name),
                            backend: Set(row.backend),
                            base_url: Set(row.base_url),
                            api_key_encrypted: Set(row.api_key_encrypted),
                            model: Set(row.model),
                            dimension: Set(row.dimension),
                            batch_size: Set(row.batch_size),
                            local_model_path: Set(row.local_model_path),
                            is_default: Set(row.is_default),
                            created_at: Set(now),
                            updated_at: Set(now),
                            ..Default::default()
                        }
                        .insert(&txn)
                        .await
                        .map_err(|e| e.to_string())?;
                        imported += 1;
                    }
                }
            }
            details.push(format!("嵌入配置：导入 {imported}，跳过 {skipped}"));
        }

        {
            let rows = incoming.image_configs;
            let mut imported = 0usize;
            let mut skipped = 0usize;
            for row in rows {
                let existing = image_gen_config::Entity::find()
                    .filter(image_gen_config::Column::Name.eq(&row.name))
                    .one(&txn)
                    .await
                    .map_err(|e| e.to_string())?;
                match (existing, overwrite) {
                    (Some(_), false) => skipped += 1,
                    (Some(e), true) => {
                        if row.is_default {
                            clear_image_defaults(&txn).await?;
                        }
                        let a = image_active_model_for_update(e.into(), row, now);
                        a.update(&txn).await.map_err(|e| e.to_string())?;
                        imported += 1;
                    }
                    (None, _) => {
                        if row.is_default {
                            clear_image_defaults(&txn).await?;
                        }
                        image_active_model_for_insert(row, now)
                            .insert(&txn)
                            .await
                            .map_err(|e| e.to_string())?;
                        imported += 1;
                    }
                }
            }
            details.push(format!("图片配置：导入 {imported}，跳过 {skipped}"));
        }

        {
            import_counted_rows!(
                details,
                "视频配置",
                incoming.video_configs,
                import_video_row,
                &txn,
                overwrite,
                now
            );
        }

        {
            details.push(format!("提示词：导入 {prompt_imported}"));
        }

        {
            import_counted_rows!(
                details,
                "主题",
                incoming.themes,
                import_theme_row,
                &txn,
                overwrite,
                now
            );
        }

        {
            let rows = incoming.system_settings;
            let mut imported = 0usize;
            let mut skipped = 0usize;
            for row in rows {
                if row.key == crate::dynamic_llm::LLM_MAX_TOKENS_SETTING_KEY {
                    validate_llm_max_tokens_import_against_current_default(&txn, row.value.clone())
                        .await?;
                }
                let existing = system_settings::Entity::find()
                    .filter(system_settings::Column::Key.eq(&row.key))
                    .one(&txn)
                    .await
                    .map_err(|e| e.to_string())?;
                match (existing, overwrite) {
                    (Some(_), false) => skipped += 1,
                    (Some(e), true) => {
                        let mut a: system_settings::ActiveModel = e.into();
                        a.value = Set(row.value);
                        a.category = Set(row.category);
                        a.description = Set(row.description);
                        a.updated_at = Set(now);
                        a.update(&txn).await.map_err(|e| e.to_string())?;
                        imported += 1;
                    }
                    (None, _) => {
                        system_settings::ActiveModel {
                            key: Set(row.key),
                            value: Set(row.value),
                            category: Set(row.category),
                            description: Set(row.description),
                            updated_at: Set(now),
                            ..Default::default()
                        }
                        .insert(&txn)
                        .await
                        .map_err(|e| e.to_string())?;
                        imported += 1;
                    }
                }
            }
            details.push(format!("系统设置：导入 {imported}，跳过 {skipped}"));
        }

        txn.commit().await.map_err(|e| e.to_string())?;

        Ok(ImportAllResult {
            success: true,
            message: "全量导入完成".to_string(),
            details,
        })
    }
    .await;

    match db_import_result {
        Ok(result) => Ok(result),
        Err(err) => {
            Err(
                rollback_prompt_import_after_db_error(state.engine.as_ref(), prompt_snapshots, err)
                    .await,
            )
        }
    }
}

async fn import_prompt_rows(
    engine: &dyn PromptEngine,
    rows: Vec<PromptRow>,
) -> Result<usize, String> {
    if rows.is_empty() {
        return Ok(0);
    }

    let count = rows.len();
    let updates = rows
        .into_iter()
        .map(|row| (row.name, row.content))
        .collect::<Vec<_>>();
    engine
        .update_many(updates, "settings_import")
        .await
        .map_err(|err| format!("提示词批量导入失败：{err}"))?;
    Ok(count)
}

fn prompt_import_snapshots(
    engine: &dyn PromptEngine,
    rows: &[PromptRow],
) -> Result<Vec<(String, String)>, String> {
    rows.iter()
        .map(|row| {
            let entry = engine
                .get(&row.name)
                .map_err(|_| format!("提示词 {} 导入失败：注册表中不存在", row.name))?;
            Ok((row.name.clone(), entry.content))
        })
        .collect()
}

async fn rollback_prompt_import_after_db_error(
    engine: &dyn PromptEngine,
    snapshots: Vec<(String, String)>,
    db_error: String,
) -> String {
    if snapshots.is_empty() {
        return db_error;
    }

    match rollback_prompt_import(engine, snapshots).await {
        Ok(()) => format!("{db_error}；已回滚本次提示词导入"),
        Err(rollback_error) => {
            format!("{db_error}；提示词已更新但回滚失败：{rollback_error}")
        }
    }
}

async fn rollback_prompt_import(
    engine: &dyn PromptEngine,
    snapshots: Vec<(String, String)>,
) -> Result<(), String> {
    if snapshots.is_empty() {
        return Ok(());
    }

    engine
        .update_many(snapshots, "settings_import_rollback")
        .await
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn validate_prompt_import_targets(
    incoming: &AllConfigsExport,
    engine: &dyn PromptEngine,
) -> Result<(), String> {
    for row in &incoming.prompts_modified {
        engine
            .get(&row.name)
            .map_err(|_| format!("提示词 {} 导入失败：注册表中不存在", row.name))?;
        validate_prompt_template(&row.name, &row.content)?;
    }
    Ok(())
}

fn validate_prompt_template(name: &str, content: &str) -> Result<(), String> {
    let mut tera = Tera::default();
    tera.add_raw_template(name, content)
        .map_err(|e| format!("提示词 {name} 模板编译失败：{e}"))?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DefaultStateRow {
    name: String,
    is_default: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DefaultImportPolicy {
    Required,
    PreserveIfConfigured,
}

async fn validate_config_default_import_results(
    db: &DatabaseConnection,
    incoming: &AllConfigsExport,
    overwrite: bool,
) -> Result<(), String> {
    let existing_llm = llm_config::Entity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|row| DefaultStateRow {
            name: row.name,
            is_default: row.is_default,
        })
        .collect::<Vec<_>>();
    ensure_single_default_after_import(
        "默认 LLM 配置",
        &existing_llm,
        &incoming.llm_configs,
        overwrite,
        DefaultImportPolicy::PreserveIfConfigured,
        |row| row.name.as_str(),
        |row| row.is_default,
    )?;

    let existing_embedding = embedding_config::Entity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|row| DefaultStateRow {
            name: row.name,
            is_default: row.is_default,
        })
        .collect::<Vec<_>>();
    ensure_single_default_after_import(
        "默认 Embedding 配置",
        &existing_embedding,
        &incoming.embedding_configs,
        overwrite,
        DefaultImportPolicy::Required,
        |row| row.name.as_str(),
        |row| row.is_default,
    )?;

    let existing_image = image_gen_config::Entity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|row| DefaultStateRow {
            name: row.name,
            is_default: row.is_default,
        })
        .collect::<Vec<_>>();
    ensure_single_default_after_import(
        "默认图片配置",
        &existing_image,
        &incoming.image_configs,
        overwrite,
        DefaultImportPolicy::PreserveIfConfigured,
        |row| row.name.as_str(),
        |row| row.is_default,
    )?;

    let existing_video = video_gen_config::Entity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|row| DefaultStateRow {
            name: row.name,
            is_default: row.is_default,
        })
        .collect::<Vec<_>>();
    ensure_single_default_after_import(
        "默认视频配置",
        &existing_video,
        &incoming.video_configs,
        overwrite,
        DefaultImportPolicy::PreserveIfConfigured,
        |row| row.name.as_str(),
        |row| row.is_default,
    )
}

fn ensure_single_default_after_import<T>(
    kind: &str,
    existing: &[DefaultStateRow],
    incoming: &[T],
    overwrite: bool,
    policy: DefaultImportPolicy,
    name: impl Fn(&T) -> &str,
    is_default: impl Fn(&T) -> bool,
) -> Result<(), String> {
    let mut result = existing.to_vec();
    let existing_default_count = result.iter().filter(|row| row.is_default).count();
    let incoming_default_count = incoming.iter().filter(|row| is_default(*row)).count();

    for row in incoming {
        let row_name = name(row);
        let row_is_default = is_default(row);
        let applied_index = match result.iter().position(|existing| existing.name == row_name) {
            Some(index) if overwrite => {
                result[index].is_default = row_is_default;
                Some(index)
            }
            Some(_) => None,
            None => {
                result.push(DefaultStateRow {
                    name: row_name.to_string(),
                    is_default: row_is_default,
                });
                Some(result.len() - 1)
            }
        };

        if let Some(default_index) = applied_index.filter(|_| row_is_default) {
            for (index, item) in result.iter_mut().enumerate() {
                item.is_default = index == default_index;
            }
        }
    }

    let default_count = result.iter().filter(|row| row.is_default).count();
    match default_count {
        1 => Ok(()),
        0 if policy == DefaultImportPolicy::PreserveIfConfigured
            && existing_default_count == 0
            && incoming_default_count == 0 =>
        {
            Ok(())
        }
        0 => Err(format!(
            "全量导入后必须保留一个{kind}，请在导入文件中设置一个默认项或先设置数据库默认项"
        )),
        count => Err(format!("全量导入后只能保留一个{kind}，当前为 {count} 个")),
    }
}

fn image_active_model_for_update(
    mut active: image_gen_config::ActiveModel,
    row: ImageRow,
    now: chrono::NaiveDateTime,
) -> image_gen_config::ActiveModel {
    active.backend = Set(row.backend);
    active.base_url = Set(row.base_url);
    active.api_key_encrypted = Set(row.api_key_encrypted);
    active.model = Set(row.model);
    active.max_concurrent = Set(row.max_concurrent);
    active.default_width = Set(row.default_width);
    active.default_height = Set(row.default_height);
    active.provider_options_json = Set(row.provider_options_json);
    active.is_default = Set(row.is_default);
    active.updated_at = Set(now);
    active
}

fn image_active_model_for_insert(
    row: ImageRow,
    now: chrono::NaiveDateTime,
) -> image_gen_config::ActiveModel {
    image_gen_config::ActiveModel {
        name: Set(row.name),
        backend: Set(row.backend),
        base_url: Set(row.base_url),
        api_key_encrypted: Set(row.api_key_encrypted),
        model: Set(row.model),
        max_concurrent: Set(row.max_concurrent),
        default_width: Set(row.default_width),
        default_height: Set(row.default_height),
        provider_options_json: Set(row.provider_options_json),
        is_default: Set(row.is_default),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

async fn validate_llm_max_tokens_import_against_current_default<C>(
    db: &C,
    value: serde_json::Value,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let config: LlmMaxTokensConfig =
        serde_json::from_value(value).map_err(|err| format!("llm_max_tokens 解析失败：{err}"))?;
    config
        .validate()
        .map_err(|err| format!("llm_max_tokens 无效：{err}"))?;
    let Some(default_model) = llm_config::Entity::find()
        .filter(llm_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(|err| err.to_string())?
    else {
        return Ok(());
    };
    let model_cap = usize::try_from(default_model.max_output_tokens)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "默认 LLM max_output_tokens 必须大于 0".to_string())?;
    config
        .validate_against_model_output_cap(model_cap)
        .map_err(|err| format!("llm_max_tokens 无效：{err}"))
}

async fn import_video_row<C>(
    db: &C,
    row: VideoRow,
    overwrite: bool,
    now: chrono::NaiveDateTime,
) -> Result<bool, String>
where
    C: ConnectionTrait,
{
    let existing = video_gen_config::Entity::find()
        .filter(video_gen_config::Column::Name.eq(&row.name))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;

    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(e), true) => {
            if row.is_default {
                clear_video_defaults(db).await?;
            }
            let a = video_active_model_for_update(e.into(), row, now);
            a.update(db).await.map_err(|e| e.to_string())?;
            Ok(true)
        }
        (None, _) => {
            if row.is_default {
                clear_video_defaults(db).await?;
            }
            video_active_model_for_insert(row, now)
                .insert(db)
                .await
                .map_err(|e| e.to_string())?;
            Ok(true)
        }
    }
}

fn video_active_model_for_update(
    mut active: video_gen_config::ActiveModel,
    row: VideoRow,
    now: chrono::NaiveDateTime,
) -> video_gen_config::ActiveModel {
    active.backend = Set(row.backend);
    active.base_url = Set(row.base_url);
    active.api_key_encrypted = Set(row.api_key_encrypted);
    active.model = Set(row.model);
    active.max_concurrent = Set(row.max_concurrent);
    active.default_width = Set(row.default_width);
    active.default_height = Set(row.default_height);
    active.default_duration_secs = Set(row.default_duration_secs);
    active.min_duration_secs = Set(row.min_duration_secs);
    active.max_duration_secs = Set(row.max_duration_secs);
    active.duration_mode = Set(row.duration_mode);
    active.supported_durations_json = Set(row.supported_durations_json);
    active.default_fps = Set(row.default_fps);
    active.provider_options_json = Set(row.provider_options_json);
    active.is_default = Set(row.is_default);
    active.updated_at = Set(now);
    active
}

fn video_active_model_for_insert(
    row: VideoRow,
    now: chrono::NaiveDateTime,
) -> video_gen_config::ActiveModel {
    video_gen_config::ActiveModel {
        name: Set(row.name),
        backend: Set(row.backend),
        base_url: Set(row.base_url),
        api_key_encrypted: Set(row.api_key_encrypted),
        model: Set(row.model),
        max_concurrent: Set(row.max_concurrent),
        default_width: Set(row.default_width),
        default_height: Set(row.default_height),
        default_duration_secs: Set(row.default_duration_secs),
        min_duration_secs: Set(row.min_duration_secs),
        max_duration_secs: Set(row.max_duration_secs),
        duration_mode: Set(row.duration_mode),
        supported_durations_json: Set(row.supported_durations_json),
        default_fps: Set(row.default_fps),
        provider_options_json: Set(row.provider_options_json),
        is_default: Set(row.is_default),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

async fn import_theme_row<C>(
    db: &C,
    row: ThemeRow,
    overwrite: bool,
    now: chrono::NaiveDateTime,
) -> Result<bool, String>
where
    C: ConnectionTrait,
{
    let mut q = theme_config::Entity::find().filter(theme_config::Column::Name.eq(&row.name));
    q = match row.user_id {
        Some(uid) => q.filter(theme_config::Column::UserId.eq(uid)),
        None => q.filter(theme_config::Column::UserId.is_null()),
    };
    let existing = q.one(db).await.map_err(|e| e.to_string())?;

    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(e), true) => {
            let mut a: theme_config::ActiveModel = e.into();
            a.mode = Set(row.mode);
            a.css_variables = Set(row.css_variables);
            a.is_system_default = Set(row.is_system_default);
            a.is_active = Set(row.is_active);
            a.updated_at = Set(now);
            let updated = a.update(db).await.map_err(|e| e.to_string())?;
            normalize_theme_activation_flags(db, row.user_id, updated.id, row.is_active).await?;
            normalize_theme_default_flags(db, updated.id, row.is_system_default).await?;
            Ok(true)
        }
        (None, _) => {
            let inserted = theme_config::ActiveModel {
                user_id: Set(row.user_id),
                name: Set(row.name),
                mode: Set(row.mode),
                css_variables: Set(row.css_variables),
                is_system_default: Set(row.is_system_default),
                is_active: Set(row.is_active),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|e| e.to_string())?;
            normalize_theme_activation_flags(db, row.user_id, inserted.id, row.is_active).await?;
            normalize_theme_default_flags(db, inserted.id, row.is_system_default).await?;
            Ok(true)
        }
    }
}

async fn normalize_theme_activation_flags<C>(
    db: &C,
    user_id: Option<i64>,
    active_theme_id: i64,
    is_active: bool,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    if !is_active {
        return Ok(());
    }

    let mut update = theme_config::Entity::update_many()
        .col_expr(
            theme_config::Column::IsActive,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(theme_config::Column::IsActive.eq(true))
        .filter(theme_config::Column::Id.ne(active_theme_id));
    update = match user_id {
        Some(uid) => update.filter(theme_config::Column::UserId.eq(uid)),
        None => update.filter(theme_config::Column::UserId.is_null()),
    };
    update.exec(db).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn normalize_theme_default_flags<C>(
    db: &C,
    default_theme_id: i64,
    is_system_default: bool,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    if !is_system_default {
        return Ok(());
    }

    theme_config::Entity::update_many()
        .col_expr(
            theme_config::Column::IsSystemDefault,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(theme_config::Column::IsSystemDefault.eq(true))
        .filter(theme_config::Column::Id.ne(default_theme_id))
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

macro_rules! clear_config_defaults {
    ($fn_name:ident, $entity:ident) => {
        async fn $fn_name<C>(db: &C) -> Result<(), String>
        where
            C: ConnectionTrait,
        {
            $entity::Entity::update_many()
                .col_expr(
                    $entity::Column::IsDefault,
                    sea_orm::sea_query::Expr::value(false),
                )
                .filter($entity::Column::IsDefault.eq(true))
                .exec(db)
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
    };
}

clear_config_defaults!(clear_llm_defaults, llm_config);
clear_config_defaults!(clear_embedding_defaults, embedding_config);
clear_config_defaults!(clear_image_defaults, image_gen_config);
clear_config_defaults!(clear_video_defaults, video_gen_config);

#[cfg(test)]
mod tests;
