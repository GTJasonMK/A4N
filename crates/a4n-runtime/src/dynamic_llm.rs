//! 动态 LLM Provider：每次调用时从数据库解析当前激活的 LLM 配置。

use std::{collections::HashMap, pin::Pin, sync::Arc};

use a4n_capabilities::LlmClient;
use a4n_core::config::app_config::{
    DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS, LlmMaxTokensConfig,
};
use a4n_core::error::AppError;
use a4n_core::models::ai::llm_config;
use a4n_llm::context::LlmContextManifest;
use a4n_llm::context::{
    COMPRESSION_FULL_RAW, LlmContextPackDraft, LlmContextPackDraftInput, LlmContextPackRef,
    LlmContextPackRequirement, PlannedContextPack, context_pack_model_ref,
};
use a4n_llm::provider::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LlmProvider, LlmTraceContext, MessageRole,
    ModelCapabilities, UsageInfo,
};
#[cfg(feature = "embedded-llm-provider")]
use a4n_llm_http::http::HttpLlmProvider;
use a4n_orchestrator::llm_context::RecordOrReuseContextPackInput;
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind, ServiceError};
use async_trait::async_trait;
use futures::stream::Stream;
use serde_json::json;

mod budget;
mod config;
mod context;
mod contracts;
mod llm_provider;
mod provider;
mod streams;
mod trace;

#[cfg(all(feature = "embedded-llm-provider", not(test)))]
use self::budget::ensure_max_tokens_within_model_cap;
#[cfg(test)]
use self::budget::{ensure_max_tokens_within_model_cap, llm_probe_succeeded};
#[cfg(feature = "embedded-llm-provider")]
use self::streams::QueueTrackedLlmStream;
pub use self::{
    budget::resolve_effective_llm_preview_budget_for_config,
    contracts::{DynamicLlmProvider, EffectiveLlmPreviewBudget, LLM_MAX_TOKENS_SETTING_KEY},
};
use self::{
    config::*,
    contracts::{LlmProbeOutput, ResolvedLlmModel},
};

#[cfg(test)]
mod tests;
