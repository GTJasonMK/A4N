//! 配置与主题 facade。

use super::*;

impl OrchestratorContext {
    pub(crate) async fn list_themes_visible_to_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        self.settings.list_themes_visible_to_user(user_id).await
    }

    pub(crate) async fn theme_by_id(
        &self,
        theme_id: i64,
    ) -> Result<Option<theme_config::Model>, AppError> {
        self.settings.theme_by_id(theme_id).await
    }

    pub(crate) async fn create_user_theme_record(
        &self,
        user_id: i64,
        name: String,
        mode: ThemeMode,
        document: ThemeDocument,
    ) -> Result<theme_config::Model, AppError> {
        self.settings
            .create_theme_record(Some(user_id), &name, mode, document)
            .await
    }

    pub(crate) async fn update_theme_record(
        &self,
        theme: theme_config::Model,
        name: Option<String>,
        mode: Option<ThemeMode>,
        document: Option<ThemeDocument>,
    ) -> Result<theme_config::Model, AppError> {
        self.settings
            .update_theme_record(theme, name.as_deref(), mode, document)
            .await
    }

    pub(crate) async fn delete_theme_record(&self, theme_id: i64) -> Result<(), AppError> {
        self.settings.delete_theme_record(theme_id).await
    }

    pub(crate) async fn activate_theme(
        &self,
        theme_id: i64,
    ) -> Result<theme_config::Model, AppError> {
        self.settings.activate_theme(theme_id).await
    }

    pub(crate) async fn list_theme_resources(
        &self,
        theme_id: i64,
    ) -> Result<Vec<a4n_db_models::models::user::theme_resource::Model>, AppError> {
        self.settings.list_theme_resources(theme_id).await
    }

    pub(crate) async fn theme_resource_by_id(
        &self,
        resource_id: i64,
    ) -> Result<Option<a4n_db_models::models::user::theme_resource::Model>, AppError> {
        self.settings.theme_resource_by_id(resource_id).await
    }

    pub(crate) async fn upsert_theme_resource(
        &self,
        theme_id: i64,
        resource_key: &str,
        file_name: &str,
        mime_type: &str,
        file_path: &str,
        sha256: &str,
        byte_size: i64,
    ) -> Result<a4n_db_models::models::user::theme_resource::Model, AppError> {
        self.settings
            .upsert_theme_resource(
                theme_id,
                resource_key,
                file_name,
                mime_type,
                file_path,
                sha256,
                byte_size,
            )
            .await
    }

    pub(crate) async fn delete_theme_resource(
        &self,
        resource_id: i64,
    ) -> Result<a4n_db_models::models::user::theme_resource::Model, AppError> {
        self.settings.delete_theme_resource(resource_id).await
    }

    pub(crate) async fn upsert_theme_preference(
        &self,
        user_id: i64,
        appearance: a4n_domain::ThemeAppearance,
    ) -> Result<a4n_db_models::models::user::theme_preference::Model, AppError> {
        self.settings
            .upsert_theme_preference(user_id, appearance)
            .await
    }

    pub(crate) async fn list_llm_config_records(&self) -> Result<Vec<llm_config::Model>, AppError> {
        self.settings.list_llm_config_records().await
    }

    pub(crate) async fn llm_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<llm_config::Model, AppError> {
        self.settings.llm_config_by_id(config_id).await
    }

    /// 创建 LLM 配置：规范化后落库。
    pub(crate) async fn create_llm_config_validated(
        &self,
        input: CreateLlmInput,
    ) -> Result<llm_config::Model, AppError> {
        self.settings.create_llm_config_validated(input).await
    }

    /// 更新 LLM 配置：规范化、默认 max_tokens 门禁后落库。
    pub(crate) async fn update_llm_config_validated(
        &self,
        config_id: i64,
        input: UpdateLlmInput,
    ) -> Result<llm_config::Model, AppError> {
        self.settings
            .update_llm_config_validated(config_id, input)
            .await
    }

    /// 设为默认 LLM 配置：模型与 max_tokens 门禁后落库。
    pub(crate) async fn set_default_llm_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .set_default_llm_config_validated(config_id)
            .await
    }

    /// 删除 LLM 配置：非默认门禁后落库。
    pub(crate) async fn delete_llm_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings.delete_llm_config_if_allowed(config_id).await
    }

    pub(crate) async fn list_image_config_records(
        &self,
    ) -> Result<Vec<image_gen_config::Model>, AppError> {
        self.settings.list_image_config_records().await
    }

    pub(crate) async fn image_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.image_config_by_id(config_id).await
    }

    /// 创建图片配置：校验参数后落库。
    pub(crate) async fn create_image_config_validated(
        &self,
        input: ImageConfigInput,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.create_image_config_validated(input).await
    }

    /// 更新图片配置：读库、合并 api_key、校验后落库。
    pub(crate) async fn update_image_config_validated(
        &self,
        config_id: i64,
        input: UpdateImageConfigInput,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings
            .update_image_config_validated(config_id, input)
            .await
    }

    /// 设为默认图片配置：模型校验后落库。
    pub(crate) async fn set_default_image_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .set_default_image_config_validated(config_id)
            .await
    }

    /// 删除图片配置：非默认门禁后落库。
    pub(crate) async fn delete_image_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .delete_image_config_if_allowed(config_id)
            .await
    }

    pub(crate) async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.resolve_image_config(config_id).await
    }

    pub(crate) async fn video_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings.video_config_by_id(config_id).await
    }

    pub(crate) async fn list_video_config_records(
        &self,
    ) -> Result<Vec<video_gen_config::Model>, AppError> {
        self.settings.list_video_config_records().await
    }

    /// 创建视频配置：校验参数后落库。
    pub(crate) async fn create_video_config_validated(
        &self,
        input: VideoConfigInput,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings.create_video_config_validated(input).await
    }

    /// 更新视频配置：读库、合并 api_key、校验后落库。
    pub(crate) async fn update_video_config_validated(
        &self,
        config_id: i64,
        input: UpdateVideoConfigInput,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings
            .update_video_config_validated(config_id, input)
            .await
    }

    /// 设为默认视频配置：模型校验后落库。
    pub(crate) async fn set_default_video_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .set_default_video_config_validated(config_id)
            .await
    }

    /// 删除视频配置：非默认且无进行中任务门禁后落库。
    pub(crate) async fn delete_video_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .delete_video_config_if_allowed(config_id)
            .await
    }

    pub(crate) async fn default_video_config(&self) -> Result<video_gen_config::Model, AppError> {
        self.settings.default_video_config().await
    }

    pub(crate) async fn active_generated_video_ids_for_config(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        self.settings
            .active_generated_video_ids_for_config(config_id)
            .await
    }

    pub(crate) async fn list_embedding_configs(
        &self,
    ) -> Result<Vec<embedding_config::Model>, AppError> {
        self.settings.list_embedding_configs().await
    }

    pub(crate) async fn embedding_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings.embedding_config_by_id(config_id).await
    }

    /// 创建嵌入配置：校验参数后落库。
    pub(crate) async fn create_embedding_config_validated(
        &self,
        input: CreateEmbeddingInput,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings.create_embedding_config_validated(input).await
    }

    /// 更新嵌入配置：完整替换字段并保留未提交的 API Key。
    pub(crate) async fn update_embedding_config_validated(
        &self,
        config_id: i64,
        input: UpdateEmbeddingInput,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings
            .update_embedding_config_validated(config_id, input)
            .await
    }

    /// 设为默认嵌入配置：模型校验后落库。
    pub(crate) async fn set_default_embedding_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .set_default_embedding_config_validated(config_id)
            .await
    }

    /// 删除嵌入配置：非默认门禁后落库。
    pub(crate) async fn delete_embedding_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .delete_embedding_config_if_allowed(config_id)
            .await
    }

    pub(crate) async fn llm_configs_exist(&self) -> Result<bool, AppError> {
        self.settings.llm_configs_exist().await
    }

    pub(crate) async fn image_configs_exist(&self) -> Result<bool, AppError> {
        self.settings.image_configs_exist().await
    }

    pub(crate) async fn video_configs_exist(&self) -> Result<bool, AppError> {
        self.settings.video_configs_exist().await
    }

    pub(crate) async fn insert_baseline_embedding_config_if_empty(&self) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_embedding_config_if_empty()
            .await
    }

    pub(crate) async fn insert_baseline_llm_config_if_empty(
        &self,
        config: NormalizedLlmConfig,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_llm_config_if_empty(config)
            .await
    }

    pub(crate) async fn insert_baseline_image_config_if_empty(
        &self,
        config: CreateImageConfigParams,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_image_config_if_empty(config)
            .await
    }

    pub(crate) async fn insert_baseline_video_config_if_empty(
        &self,
        config: VideoConfigParams,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_video_config_if_empty(config)
            .await
    }
}

impl OrchestratorContext {
    pub(in crate::context) async fn require_settings_admin(
        &self,
        user_id: i64,
    ) -> Result<(), AppError> {
        self.require_admin_permission(
            user_id,
            a4n_api_contract::admin::AdminPermission::ManageSettings,
        )
        .await
    }
}
