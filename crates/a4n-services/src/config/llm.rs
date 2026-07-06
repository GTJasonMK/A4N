use chrono::Utc;
use sea_orm::{ActiveModelTrait, QueryOrder, Set};
use serde_json::Value;

use a4n_core::error::AppError;
use a4n_core::models::ai::llm_config;

use super::ConfigService;
use super::validation::{
    normalize_config_name, normalize_llm_request_options_json, positive_i32, required_config_value,
    validate_llm_config_model,
};

#[derive(Debug)]
pub struct CreateLlmConfigInput<'a> {
    pub name: &'a str,
    pub provider: &'a str,
    pub base_url: &'a str,
    pub api_key_encrypted: Option<&'a str>,
    pub model: &'a str,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    pub request_options_json: Option<Value>,
}

#[derive(Debug, Default)]
pub struct UpdateLlmConfigInput<'a> {
    pub name: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub api_key_encrypted: Option<Option<&'a str>>,
    pub model: Option<&'a str>,
    pub max_concurrent: Option<i32>,
    pub timeout_secs: Option<i32>,
    pub max_context_tokens: Option<i32>,
    pub max_output_tokens: Option<i32>,
    pub request_options_json: Option<Option<Value>>,
}

impl ConfigService {
    config_record_crud_methods!(
        entity = llm_config::Entity,
        model = llm_config::Model,
        list = list_llm => <llm_config::Entity as sea_orm::EntityTrait>::find()
            .order_by_desc(llm_config::Column::UpdatedAt),
        get = get_llm,
        delete = delete_llm,
        default = set_default_llm -> () => |_| (),
        not_found = |config_id| format!("LLM配置不存在: {config_id}"),
        validate = validate_llm_config_model,
        before_delete = |cfg: &llm_config::Model| super::ensure_default_config_delete_allowed(
            " LLM 配置",
            cfg.is_default,
        ),
        default_column = llm_config::Column::IsDefault,
    );

    pub async fn create_llm(
        &self,
        input: CreateLlmConfigInput<'_>,
    ) -> Result<llm_config::Model, AppError> {
        let CreateLlmConfigInput {
            name,
            provider,
            base_url,
            api_key_encrypted,
            model,
            max_concurrent,
            timeout_secs,
            max_context_tokens,
            max_output_tokens,
            request_options_json,
        } = input;
        let name = normalize_config_name(name, "LLM配置")?;
        self.ensure_config_name_available::<llm_config::Entity>(
            "LLM配置",
            &name,
            llm_config::Column::Name,
            llm_config::Column::Id,
            None,
        )
        .await?;
        let provider = required_config_value(provider, "LLM配置", "provider")?;
        let base_url = required_config_value(base_url, "LLM配置", "base_url")?;
        let model = required_config_value(model, "LLM配置", "model")?;
        let max_concurrent = positive_i32(max_concurrent, "LLM配置 max_concurrent")?;
        let timeout_secs = positive_i32(timeout_secs, "LLM配置 timeout_secs")?;
        let max_context_tokens = positive_i32(max_context_tokens, "LLM配置 max_context_tokens")?;
        let max_output_tokens = positive_i32(max_output_tokens, "LLM配置 max_output_tokens")?;
        if max_output_tokens >= max_context_tokens {
            return Err(AppError::InvalidArgument(
                "LLM配置 max_output_tokens 必须小于 max_context_tokens".into(),
            ));
        }
        let request_options_json = normalize_llm_request_options_json(request_options_json)?;
        let now = Utc::now().naive_utc();
        llm_config::ActiveModel {
            name: Set(name),
            provider: Set(provider),
            base_url: Set(base_url),
            api_key_encrypted: Set(api_key_encrypted.map(|s| s.to_string())),
            model: Set(model),
            max_concurrent: Set(max_concurrent),
            timeout_secs: Set(timeout_secs),
            max_context_tokens: Set(max_context_tokens),
            max_output_tokens: Set(max_output_tokens),
            request_options_json: Set(request_options_json),
            is_default: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn update_llm(
        &self,
        config_id: i64,
        input: UpdateLlmConfigInput<'_>,
    ) -> Result<llm_config::Model, AppError> {
        let UpdateLlmConfigInput {
            name,
            provider,
            base_url,
            api_key_encrypted,
            model,
            max_concurrent,
            timeout_secs,
            max_context_tokens,
            max_output_tokens,
            request_options_json,
        } = input;
        let cfg = self.get_llm(config_id).await?;
        let existing_max_context_tokens = cfg.max_context_tokens;
        let existing_max_output_tokens = cfg.max_output_tokens;
        let mut active: llm_config::ActiveModel = cfg.into();
        if let Some(v) = name {
            let name = normalize_config_name(v, "LLM配置")?;
            self.ensure_config_name_available::<llm_config::Entity>(
                "LLM配置",
                &name,
                llm_config::Column::Name,
                llm_config::Column::Id,
                Some(config_id),
            )
            .await?;
            active.name = Set(name);
        }
        if let Some(v) = provider {
            active.provider = Set(required_config_value(v, "LLM配置", "provider")?);
        }
        if let Some(v) = base_url {
            active.base_url = Set(required_config_value(v, "LLM配置", "base_url")?);
        }
        if let Some(v) = api_key_encrypted {
            active.api_key_encrypted = Set(v.map(|value| value.to_string()));
        }
        if let Some(v) = model {
            active.model = Set(required_config_value(v, "LLM配置", "model")?);
        }
        if let Some(v) = max_concurrent {
            active.max_concurrent = Set(positive_i32(v, "LLM配置 max_concurrent")?);
        }
        if let Some(v) = timeout_secs {
            active.timeout_secs = Set(positive_i32(v, "LLM配置 timeout_secs")?);
        }
        if let Some(v) = max_context_tokens {
            active.max_context_tokens = Set(positive_i32(v, "LLM配置 max_context_tokens")?);
        }
        if let Some(v) = max_output_tokens {
            active.max_output_tokens = Set(positive_i32(v, "LLM配置 max_output_tokens")?);
        }
        if let Some(v) = request_options_json {
            active.request_options_json = Set(normalize_llm_request_options_json(v)?);
        }
        let next_context = match &active.max_context_tokens {
            sea_orm::ActiveValue::Set(value) => *value,
            _ => existing_max_context_tokens,
        };
        let next_output = match &active.max_output_tokens {
            sea_orm::ActiveValue::Set(value) => *value,
            _ => existing_max_output_tokens,
        };
        if next_output >= next_context {
            return Err(AppError::InvalidArgument(
                "LLM配置 max_output_tokens 必须小于 max_context_tokens".into(),
            ));
        }
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }
}
