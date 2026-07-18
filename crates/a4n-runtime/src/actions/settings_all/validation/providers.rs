mod llm;
mod mappings;

use super::super::types::AllConfigsExport;
use super::shared::{trim_string, validate_default_named_rows};
use a4n_core::config::{validate_embedding_config_params, validate_image_config_params};
use a4n_llm::provider::normalize_request_options_json;
use a4n_orchestrator::video::validate_video_config_params;

use llm::{plain_error, validate_llm_rows};
use mappings::{
    apply_embedding_params_to_row, apply_image_params_to_row, apply_video_params_to_row,
    embedding_params_from_row, image_params_from_row, video_params_from_row,
};

pub(super) fn validate_provider_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_llm_rows(incoming)?;
    validate_embedding_rows(incoming)?;
    validate_image_rows(incoming)?;
    validate_video_rows(incoming)
}

pub(super) fn normalize_provider_rows(incoming: &mut AllConfigsExport) -> Result<(), String> {
    for (index, row) in incoming.llm_configs.iter_mut().enumerate() {
        let label = format!("LLM 配置 #{}", index + 1);
        trim_string(&mut row.name);
        trim_string(&mut row.provider);
        trim_string(&mut row.base_url);
        trim_string(&mut row.model);
        row.request_options_json = normalize_request_options_json(row.request_options_json.clone())
            .map_err(|err| {
                format!(
                    "{label} request_options_json 规范化失败：{}",
                    plain_error(err)
                )
            })?;
    }

    for (index, row) in incoming.embedding_configs.iter_mut().enumerate() {
        let params = validate_embedding_config_params(embedding_params_from_row(row))
            .map_err(|err| format!("嵌入配置 #{} 规范化失败：{err}", index + 1))?;
        apply_embedding_params_to_row(row, params);
    }

    for (index, row) in incoming.image_configs.iter_mut().enumerate() {
        let params = validate_image_config_params(image_params_from_row(row))
            .map_err(|err| format!("图片配置 #{} 规范化失败：{err}", index + 1))?;
        apply_image_params_to_row(row, params);
    }

    for (index, row) in incoming.video_configs.iter_mut().enumerate() {
        let params = validate_video_config_params(video_params_from_row(row))
            .map_err(|err| format!("视频配置 #{} 规范化失败：{err}", index + 1))?;
        apply_video_params_to_row(row, params);
    }
    Ok(())
}

fn validate_embedding_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_default_named_rows(
        "嵌入配置",
        &incoming.embedding_configs,
        |row| row.is_default,
        |row| &row.name,
        |label, row| {
            validate_embedding_config_params(embedding_params_from_row(row))
                .map(|_| ())
                .map_err(|err| format!("{label} {err}"))
        },
    )
}

fn validate_image_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_default_named_rows(
        "图片配置",
        &incoming.image_configs,
        |row| row.is_default,
        |row| &row.name,
        |label, row| {
            validate_image_config_params(image_params_from_row(row))
                .map(|_| ())
                .map_err(|err| format!("{label} {err}"))
        },
    )
}

fn validate_video_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_default_named_rows(
        "视频配置",
        &incoming.video_configs,
        |row| row.is_default,
        |row| &row.name,
        |label, row| {
            validate_video_config_params(video_params_from_row(row))
                .map(|_| ())
                .map_err(|err| format!("{label} {err}"))
        },
    )
}
