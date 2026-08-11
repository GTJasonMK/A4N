use a4n_core::models::ai::video_gen_config;

pub(in crate::actions) async fn resolve_video_config(
    settings: &a4n_orchestrator::OrchestratorContext,
    config_id: Option<i64>,
) -> Result<video_gen_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_video_model_for_id(config_id) {
        return Ok(config);
    }
    if config_id.is_none() && crate::runtime_config::uses_system_default_video_template() {
        return resolve_system_default_video_template_with_user_key(settings).await;
    }
    if let Some(id) = config_id
        && crate::runtime_config::current_video_snapshot_scope_present()
    {
        return Err(format!(
            "runtime_config_snapshot.video_configs 不包含视频配置 #{id}"
        ));
    }
    if crate::runtime_config::current_user_id().is_some() {
        return Err(match config_id {
            Some(id) => format!(
                "远程用户请求不能直接使用服务器视频配置 #{id}；请在用户配置页使用个人视频配置快照"
            ),
            None => "使用系统默认视频配置需要先在用户配置页填写并保存你的个人 API Key".to_string(),
        });
    }

    persisted_video_config(settings, config_id).await
}

async fn resolve_system_default_video_template_with_user_key(
    settings: &a4n_orchestrator::OrchestratorContext,
) -> Result<video_gen_config::Model, String> {
    let api_key = crate::runtime_config::current_system_video_api_key().ok_or_else(|| {
        "使用系统默认视频配置需要先在用户配置页填写并保存你的个人 API Key".to_string()
    })?;
    let mut model = persisted_video_config(settings, None).await?;
    model.api_key_encrypted = Some(api_key);
    Ok(model)
}

async fn persisted_video_config(
    settings: &a4n_orchestrator::OrchestratorContext,
    config_id: Option<i64>,
) -> Result<video_gen_config::Model, String> {
    match config_id {
        Some(id) => settings.video_config(id).await,
        None => settings.default_video_config().await,
    }
    .map_err(crate::actions::app_error_message)
}
