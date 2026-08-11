use super::*;

pub(crate) async fn resolve_image_config(
    state: &AppState,
    config_id: Option<i64>,
) -> Result<image_gen_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_image_model_for_id(config_id) {
        return Ok(config);
    }
    if config_id.is_none() && crate::runtime_config::uses_system_default_image_template() {
        return resolve_system_default_image_template_with_user_key(state).await;
    }
    if let Some(id) = config_id
        && crate::runtime_config::current_image_snapshot_scope_present()
    {
        return Err(format!(
            "runtime_config_snapshot.image_configs 不包含图片配置 #{id}"
        ));
    }
    if crate::runtime_config::current_user_id().is_some() {
        return Err(match config_id {
            Some(id) => format!(
                "远程用户请求不能直接使用服务器图片配置 #{id}；请在用户配置页使用个人图片配置快照"
            ),
            None => "使用系统默认图片配置需要先在用户配置页填写并保存你的个人 API Key".to_string(),
        });
    }

    state
        .orchestrator()
        .resolve_image_config(config_id)
        .await
        .map_err(|err| err.to_string())
}

pub(super) async fn resolve_system_default_image_template_with_user_key(
    state: &AppState,
) -> Result<image_gen_config::Model, String> {
    let mut model = state
        .orchestrator()
        .resolve_image_config(None)
        .await
        .map_err(|err| err.to_string())?;
    model.api_key_encrypted = crate::runtime_config::current_system_image_api_key();
    if model.api_key_encrypted.is_none()
        && a4n_orchestrator::config::image_backend_requires_api_key(&model.backend)
    {
        return Err("使用系统默认图片配置需要先在用户配置页填写并保存你的个人 API Key".to_string());
    }
    Ok(model)
}
