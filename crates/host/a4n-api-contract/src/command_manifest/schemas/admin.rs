use super::builders::*;
use serde_json::{Value, json};

pub(crate) fn args_activate_theme() -> Value {
    json!({
        "type": "object",
        "properties": { "themeId": { "type": "integer", "minimum": 1 } },
        "required": ["themeId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_delete_theme_resource() -> Value {
    strict_typed_input_schema::<crate::theme::DeleteThemeResourceInput>()
}
pub(crate) fn args_theme_resource_upload() -> Value {
    strict_typed_input_schema::<crate::theme::ThemeResourceUploadInput>()
}
pub(crate) fn args_theme_package_upload() -> Value {
    strict_typed_input_schema::<crate::theme::ThemePackageUploadInput>()
}
pub(crate) fn args_update_theme_preference() -> Value {
    strict_typed_input_schema::<crate::theme::UpdateThemePreferenceInput>()
}
pub(crate) fn args_add_image_config() -> Value {
    strict_typed_input_schema::<crate::config::ImageConfigInput>()
}
pub(crate) fn args_add_video_config() -> Value {
    strict_typed_input_schema::<crate::config::VideoConfigInput>()
}
pub(crate) fn args_cancel_embedding_model_download() -> Value {
    typed_input_schema::<crate::embedding::CancelEmbeddingModelDownloadInput>()
}
pub(crate) fn args_cancel_generation_task() -> Value {
    strict_typed_input_schema::<crate::generation_tasks::CancelGenerationTaskInput>()
}
pub(crate) fn args_cancel_task() -> Value {
    typed_schema::<crate::task::CancelTaskInput>()
}
pub(crate) fn args_cancel_workflow() -> Value {
    strict_typed_input_schema::<crate::workflow::CancelWorkflowInput>()
}
pub(crate) fn args_change_user_password() -> Value {
    typed_input_schema::<crate::auth::ChangePasswordInput>()
}
pub(crate) fn args_create_embedding_config() -> Value {
    strict_typed_input_schema::<crate::config::CreateEmbeddingInput>()
}
pub(crate) fn args_create_llm_config() -> Value {
    strict_typed_input_schema::<crate::config::CreateLlmInput>()
}
pub(crate) fn args_create_theme() -> Value {
    strict_typed_input_schema::<crate::theme::CreateThemeInput>()
}
pub(crate) fn args_create_invite_code() -> Value {
    strict_typed_input_schema::<crate::admin::CreateInviteCodeInput>()
}
pub(crate) fn args_create_user() -> Value {
    strict_typed_input_schema::<crate::admin::CreateUserInput>()
}
pub(crate) fn args_cleanup_orphan_project_data_dirs() -> Value {
    strict_typed_input_schema::<crate::storage::CleanupOrphanProjectDataDirsInput>()
}
pub(crate) fn args_delete_local_embedding_model() -> Value {
    typed_input_schema::<crate::embedding::EmbeddingModelInput>()
}
pub(crate) fn args_oauth2_authorize_url() -> Value {
    strict_typed_input_schema::<crate::auth::OAuth2AuthorizeInput>()
}
pub(crate) fn args_oauth2_available_providers() -> Value {
    strict_typed_input_schema::<crate::auth::OAuth2AvailableProvidersInput>()
}
pub(crate) fn args_oauth2_callback() -> Value {
    strict_typed_input_schema::<crate::auth::OAuth2CallbackInput>()
}
pub(crate) fn result_oauth2_available_providers() -> Value {
    typed_schema::<Vec<String>>()
}
pub(crate) fn args_rag_enhanced_search() -> Value {
    project_runtime_typed_input_schema::<crate::rag::EnhancedSearchInput>()
}
pub(crate) fn args_rag_enhanced_search_with_vectors() -> Value {
    project_runtime_typed_input_schema::<crate::rag::RagEnhancedSearchWithVectorsInput>()
}
pub(crate) fn args_rag_ingest_all() -> Value {
    project_optional_task_runtime_typed_input_schema::<crate::rag::RagIngestOptions>()
}
pub(crate) fn args_rag_prepare_ingest_all() -> Value {
    project_runtime_typed_input_schema::<crate::rag::RagIngestOptions>()
}
pub(crate) fn args_rag_upload_ingest_embeddings() -> Value {
    project_runtime_typed_input_schema::<crate::rag::ClientRagIngestUploadInput>()
}
pub(crate) fn args_detect_image_config_models() -> Value {
    strict_typed_input_schema::<crate::config::ModelDiscoveryInput>()
}
pub(crate) fn args_download_embedding_model() -> Value {
    typed_input_schema::<crate::embedding::DownloadEmbeddingModelInput>()
}
pub(crate) fn args_respond_agent_editor() -> Value {
    strict_typed_input_schema::<crate::agent::RespondAgentEditorInput>()
}
pub(crate) fn result_agent_editor_reply() -> Value {
    typed_schema::<crate::agent::AgentEditorReply>()
}
pub(crate) fn args_review_agent_delegation() -> Value {
    strict_typed_input_schema::<crate::agent::ReviewAgentDelegationInput>()
}
pub(crate) fn args_update_agent_skill_configuration() -> Value {
    strict_typed_input_schema::<crate::agent::UpdateAgentSkillConfigurationInput>()
}
pub(crate) fn result_agent_delegation() -> Value {
    typed_schema::<crate::agent::AgentDelegationView>()
}
pub(crate) fn result_agent_skill_profile() -> Value {
    typed_schema::<crate::agent::AgentSkillProfileView>()
}
pub(crate) fn result_agent_skill_settings() -> Value {
    typed_schema::<crate::agent::AgentSkillSettingsView>()
}
pub(crate) fn args_export_batch_data() -> Value {
    required_task_typed_input_schema::<crate::export::BatchExportInput>()
}
pub(crate) fn args_export_all_configs_json() -> Value {
    empty_args_schema()
}
pub(crate) fn args_export_characters_json_data() -> Value {
    required_task_typed_input_schema::<crate::export::ExportProjectInput>()
}
pub(crate) fn args_export_epub_data() -> Value {
    required_task_typed_input_schema::<crate::export::ExportPublicationInput>()
}
pub(crate) fn args_get_generation_call_analytics() -> Value {
    strict_typed_input_schema::<crate::admin::GenerationCallAnalyticsInput>()
}
pub(crate) fn args_get_model_usage_report() -> Value {
    strict_typed_input_schema::<crate::cost::ModelUsageReportInput>()
}
pub(crate) fn args_get_model_usage_request() -> Value {
    strict_typed_input_schema::<crate::cost::ModelUsageRequestInput>()
}
pub(crate) fn args_get_package_operation() -> Value {
    strict_typed_input_schema::<crate::package::GetPackageOperationInput>()
}
pub(crate) fn args_get_task_history() -> Value {
    typed_schema::<crate::task::GetTaskHistoryInput>()
}
pub(crate) fn args_user_usage() -> Value {
    json!({
        "type": "object",
        "properties": {
            "userId": { "type": "integer", "minimum": 1 },
            "limit": { "type": "integer", "minimum": 1 },
        },
        "required": ["userId", "limit"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_get_usage_trends() -> Value {
    strict_typed_input_schema::<crate::admin::UsageTrendsInput>()
}
pub(crate) fn args_get_workflow_progress_events() -> Value {
    strict_typed_input_schema::<crate::workflow::WorkflowProgressEventsInput>()
}

pub(crate) fn args_start_test_run() -> Value {
    strict_typed_input_schema::<crate::admin::StartTestRunInput>()
}

pub(crate) fn args_list_test_runs() -> Value {
    strict_typed_input_schema::<crate::admin::ListTestRunsInput>()
}

pub(crate) fn args_test_run_id() -> Value {
    strict_typed_input_schema::<crate::admin::TestRunIdInput>()
}

pub(crate) fn args_delete_test_run() -> Value {
    strict_typed_input_schema::<crate::admin::DeleteTestRunInput>()
}
pub(crate) fn args_get_workflow_run_detail() -> Value {
    strict_typed_input_schema::<crate::workflow::GetWorkflowRunInput>()
}
pub(crate) fn args_import_all_configs() -> Value {
    typed_input_schema::<crate::config::ImportAllConfigsInput>()
}
pub(crate) fn args_invite_project_member() -> Value {
    typed_input_schema::<crate::collaboration::InviteProjectMemberInput>()
}
pub(crate) fn args_list_package_command_availability() -> Value {
    strict_typed_input_schema::<crate::package::ListPackageCommandAvailabilityInput>()
}
pub(crate) fn args_login_user() -> Value {
    typed_input_schema::<crate::auth::LoginInput>()
}
pub(crate) fn args_preview_test_embedding_config_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "configId": { "type": "integer", "minimum": 1 },
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["configId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_read_recent_logs() -> Value {
    typed_input_schema::<crate::logging::ReadLogsInput>()
}
pub(crate) fn args_list_workflow_runs() -> Value {
    strict_typed_input_schema::<crate::workflow::ListWorkflowRunsInput>()
}
pub(crate) fn args_refresh_token() -> Value {
    typed_input_schema::<crate::auth::RefreshTokenInput>()
}
pub(crate) fn args_revoke_session() -> Value {
    typed_input_schema::<crate::auth::RevokeSessionInput>()
}
pub(crate) fn args_register_user() -> Value {
    typed_input_schema::<crate::auth::RegisterInput>()
}
pub(crate) fn args_resume_workflow_run() -> Value {
    strict_runtime_typed_input_schema::<crate::workflow::ResumeWorkflowRunInput>()
}
pub(crate) fn args_set_log_level() -> Value {
    typed_input_schema::<crate::logging::SetLogLevelInput>()
}
pub(crate) fn args_start_package_lifecycle() -> Value {
    strict_typed_input_schema::<crate::package::StartPackageLifecycleInput>()
}
pub(crate) fn args_update_embedding_config() -> Value {
    config_typed_input_schema::<crate::config::UpdateEmbeddingInput>()
}
pub(crate) fn args_update_image_config() -> Value {
    config_typed_input_schema::<crate::config::UpdateImageConfigInput>()
}
pub(crate) fn args_update_llm_config() -> Value {
    config_typed_input_schema::<crate::config::UpdateLlmInput>()
}
pub(crate) fn args_web_search() -> Value {
    typed_schema::<crate::web_search::WebSearchInput>()
}
pub(crate) fn args_web_fetch() -> Value {
    typed_schema::<crate::web_fetch::WebFetchInput>()
}
pub(crate) fn args_update_package_preferences() -> Value {
    strict_typed_input_schema::<crate::package::UpdatePackagePreferencesInput>()
}
pub(crate) fn args_update_project_member_role() -> Value {
    json!({
        "type": "object",
        "properties": {
            "memberUserId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::collaboration::UpdateProjectMemberRoleInput>("input"),
        },
        "required": ["memberUserId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_update_theme() -> Value {
    json!({
        "type": "object",
        "properties": {
            "themeId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::theme::UpdateThemeInput>("input"),
        },
        "required": ["themeId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_update_user_profile() -> Value {
    typed_input_schema::<crate::auth::UpdateProfileInput>()
}
pub(crate) fn args_update_user_role() -> Value {
    json!({
        "type": "object",
        "properties": {
            "userId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<crate::admin::UpdateUserRoleInput>("input"),
        },
        "required": ["userId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
pub(crate) fn args_update_video_config() -> Value {
    config_typed_input_schema::<crate::config::UpdateVideoConfigInput>()
}
pub(crate) fn result_invite_code() -> Value {
    typed_schema::<crate::admin::InviteCodeSummary>()
}
pub(crate) fn result_invite_codes() -> Value {
    typed_schema::<Vec<crate::admin::InviteCodeSummary>>()
}
pub(crate) fn result_users() -> Value {
    typed_schema::<Vec<crate::auth::UserSummary>>()
}
pub(crate) fn result_rag_enhanced_search() -> Value {
    typed_schema::<crate::rag::RagEnhancedSearchOutput>()
}
pub(crate) fn result_rag_prepare_enhanced_search() -> Value {
    typed_schema::<crate::rag::RagPreparedEnhancedSearchOutput>()
}
pub(crate) fn result_rag_ingest() -> Value {
    typed_schema::<crate::rag::RagIngestResult>()
}
pub(crate) fn result_rag_prepare_ingest_all() -> Value {
    typed_schema::<crate::rag::ClientRagIngestPrepareOutput>()
}
pub(crate) fn result_activate_theme() -> Value {
    typed_schema::<crate::theme::ThemeSummary>()
}
pub(crate) fn result_theme_preference() -> Value {
    typed_schema::<crate::theme::ThemePreferenceSummary>()
}
pub(crate) fn result_theme_resource() -> Value {
    typed_schema::<crate::theme::ThemeResourceSummary>()
}
pub(crate) fn result_theme_package_export() -> Value {
    typed_schema::<crate::theme::ThemePackageExport>()
}
pub(crate) fn result_add_image_config() -> Value {
    typed_schema::<crate::config::ImageConfigSummary>()
}
pub(crate) fn result_add_video_config() -> Value {
    typed_schema::<crate::config::VideoConfigSummary>()
}
pub(crate) fn result_cleanup_orphan_project_data_dirs() -> Value {
    typed_schema::<crate::storage::OrphanProjectDataDirsSummary>()
}
pub(crate) fn result_create_embedding_config() -> Value {
    typed_schema::<crate::config::EmbeddingConfigSummary>()
}
pub(crate) fn result_create_llm_config() -> Value {
    typed_schema::<crate::config::LlmConfigSummary>()
}
pub(crate) fn result_detect_image_config_models() -> Value {
    typed_schema::<crate::config::ModelDiscoveryResult>()
}
pub(crate) fn result_download_embedding_model() -> Value {
    typed_schema::<crate::embedding::LocalEmbeddingModelSummary>()
}
pub(crate) fn result_export_batch_data() -> Value {
    typed_schema::<Vec<crate::export::ExportFileData>>()
}
pub(crate) fn result_export_characters_json_data() -> Value {
    typed_schema::<crate::export::ExportFileData>()
}
pub(crate) fn result_import_all_configs() -> Value {
    typed_schema::<Vec<String>>()
}
pub(crate) fn result_get_admin_overview() -> Value {
    typed_schema::<crate::admin::AdminOverviewSummary>()
}
pub(crate) fn result_get_current_user() -> Value {
    typed_schema::<crate::auth::UserSummary>()
}
pub(crate) fn result_get_embedding_model_download_info() -> Value {
    typed_schema::<crate::embedding::EmbeddingModelDownloadInfoSummary>()
}
pub(crate) fn result_get_generation_call_analytics() -> Value {
    typed_schema::<crate::admin::GenerationCallAnalytics>()
}
pub(crate) fn result_get_model_usage_report() -> Value {
    typed_schema::<crate::cost::ModelUsageReport>()
}
pub(crate) fn result_get_model_usage_request() -> Value {
    typed_schema::<crate::cost::ModelUsageRequest>()
}
pub(crate) fn result_web_search() -> Value {
    typed_schema::<crate::web_search::WebSearchResult>()
}
pub(crate) fn result_web_fetch() -> Value {
    typed_schema::<crate::web_fetch::WebFetchResult>()
}
pub(crate) fn result_package_command_availability() -> Value {
    typed_schema::<Vec<crate::package::PackageCommandAvailability>>()
}
pub(crate) fn result_package_lifecycle() -> Value {
    typed_schema::<crate::package::PackageLifecycleSummary>()
}
pub(crate) fn result_package_overview() -> Value {
    typed_schema::<Vec<crate::package::PackageOverview>>()
}
pub(crate) fn result_package_preferences() -> Value {
    typed_schema::<crate::package::PackagePreferencesSummary>()
}
pub(crate) fn result_get_queue_stats() -> Value {
    typed_schema::<crate::task::QueueStatsSummary>()
}
pub(crate) fn result_get_storage_paths() -> Value {
    typed_schema::<crate::storage::StoragePathsSummary>()
}
pub(crate) fn result_get_task_history() -> Value {
    typed_schema::<Vec<crate::task::TaskInfoSummary>>()
}
pub(crate) fn result_get_usage_trends() -> Value {
    typed_schema::<Vec<crate::admin::DailyUsagePoint>>()
}
pub(crate) fn result_get_user_usage() -> Value {
    typed_schema::<Vec<crate::admin::UsageLogSummary>>()
}
pub(crate) fn result_get_workflow_progress_events() -> Value {
    typed_schema::<crate::workflow::WorkflowProgressEventsResponse>()
}
pub(crate) fn result_get_workflow_run_detail() -> Value {
    typed_schema::<crate::workflow::WorkflowRunDetailSummary>()
}
pub(crate) fn result_list_active_workflows() -> Value {
    typed_schema::<Vec<crate::workflow::WorkflowStateSummary>>()
}
pub(crate) fn result_list_workflow_runs() -> Value {
    typed_schema::<Vec<crate::workflow::WorkflowRunSummary>>()
}
pub(crate) fn result_list_all_projects() -> Value {
    typed_schema::<Vec<crate::admin::AdminProjectSummary>>()
}
pub(crate) fn result_list_embedding_config() -> Value {
    typed_schema::<Vec<crate::config::EmbeddingConfigSummary>>()
}
pub(crate) fn result_list_image_configs() -> Value {
    typed_schema::<Vec<crate::config::ImageConfigSummary>>()
}
pub(crate) fn result_list_llm_config() -> Value {
    typed_schema::<Vec<crate::config::LlmConfigSummary>>()
}
pub(crate) fn result_list_local_embedding_models() -> Value {
    typed_schema::<Vec<crate::embedding::LocalEmbeddingModelSummary>>()
}
pub(crate) fn result_list_model_provider_catalog() -> Value {
    typed_schema::<crate::provider_catalog::ModelProviderCatalog>()
}
pub(crate) fn result_list_my_project_invites() -> Value {
    typed_schema::<Vec<crate::collaboration::ProjectInviteSummary>>()
}
pub(crate) fn result_list_pending_tasks() -> Value {
    typed_schema::<Vec<crate::admin::TaskQueueSummary>>()
}
pub(crate) fn result_test_scenarios() -> Value {
    typed_schema::<Vec<crate::admin::TestScenarioDescriptor>>()
}
pub(crate) fn result_test_run() -> Value {
    typed_schema::<crate::admin::TestRunSummary>()
}
pub(crate) fn result_test_run_detail() -> Value {
    typed_schema::<crate::admin::TestRunDetail>()
}
pub(crate) fn result_test_runs() -> Value {
    typed_schema::<Vec<crate::admin::TestRunSummary>>()
}
pub(crate) fn result_list_project_members() -> Value {
    typed_schema::<Vec<crate::collaboration::ProjectMemberSummary>>()
}
pub(crate) fn result_project_invite() -> Value {
    typed_schema::<crate::collaboration::ProjectInviteSummary>()
}
pub(crate) fn result_list_sessions() -> Value {
    typed_schema::<Vec<crate::auth::SessionSummary>>()
}
pub(crate) fn result_list_themes() -> Value {
    typed_schema::<Vec<crate::theme::ThemeSummary>>()
}
pub(crate) fn result_list_video_configs() -> Value {
    typed_schema::<Vec<crate::config::VideoConfigSummary>>()
}
pub(crate) fn result_login_user() -> Value {
    typed_schema::<crate::auth::AuthResult>()
}
pub(crate) fn result_model_request_preview() -> Value {
    typed_schema::<crate::request_preview::ModelRequestPreview>()
}
pub(crate) fn result_read_recent_logs() -> Value {
    typed_schema::<crate::logging::LogReadResult>()
}
pub(crate) fn result_refresh_token() -> Value {
    typed_schema::<crate::auth::TokenPairSummary>()
}
pub(crate) fn result_generation_task_handle() -> Value {
    typed_schema::<crate::generation_tasks::GenerationTaskHandle>()
}
pub(crate) fn result_workflow_run() -> Value {
    typed_schema::<crate::workflow::WorkflowRunSummary>()
}
