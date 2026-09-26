use std::collections::{BTreeSet, HashMap};

use a4n_error::AppError;
use serde_json::{Value, json};

use super::{LlmContextManifest, PromptRenderer, source_content_sha256};

/// 模板版本参与断点指纹，但 hash 和其他阶段的模板不是模型输入。
pub(super) async fn record_prompt_dependencies(
    manifest: &mut LlmContextManifest,
    renderer: &dyn PromptRenderer,
    project_id: i64,
    variables: &HashMap<String, Value>,
    prompt_names: &[&str],
) -> Result<(), AppError> {
    let names = prompt_names
        .iter()
        .copied()
        .chain(["video_prompt_session"])
        .collect::<BTreeSet<_>>();
    let mut dependencies = Vec::new();
    for name in names {
        let rendered = renderer
            .render_prompt(name, variables, Some(project_id))
            .await?;
        dependencies.push(json!({"prompt_name": name, "sha256": source_content_sha256(&rendered)}));
    }
    let source = manifest
        .packs
        .first_mut()
        .ok_or_else(|| AppError::InvalidArgument("视频请求缺少来源上下文".to_string()))?;
    source.config_hash = source_content_sha256(&serde_json::to_string(&json!({
        "context_config_hash": &source.config_hash,
        "prompt_dependencies": &dependencies,
    }))?);
    let refs = source.source_refs_json.get_or_insert_with(|| json!({}));
    let refs = refs
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidArgument("视频来源引用必须是对象".to_string()))?;
    refs.insert("prompt_dependencies".to_string(), json!(dependencies));
    Ok(())
}
