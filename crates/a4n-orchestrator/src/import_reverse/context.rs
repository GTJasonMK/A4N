mod blueprint;
mod chapter_entries;
mod extraction;
mod project;
#[cfg(test)]
mod test_support;

use a4n_core::import_reverse::{
    ReverseContentRequest, ReverseContextKind, required_reverse_context_limit,
};
use a4n_db::import_reverse::ReverseContextRows;
use serde_json::json;

use self::project::project_reverse_context;

pub fn resolve_reverse_content_sample(
    input: ReverseContentRequest<'_>,
    kind: ReverseContextKind,
    rows: Option<&ReverseContextRows>,
) -> Result<String, String> {
    let manual = input.manual_sample.trim();
    if !input.auto_context {
        if manual.is_empty() {
            return Err("请提供反推样本，或启用项目上下文".to_string());
        }
        return Ok(manual.to_string());
    }

    let limit = required_reverse_context_limit(input.context_limit)?;
    let rows = rows.ok_or_else(|| "启用项目上下文时缺少反推上下文行".to_string())?;
    let mut context = project_reverse_context(rows, input.project_id, kind, limit)?;
    if !manual.is_empty() {
        if let Some(obj) = context.as_object_mut() {
            obj.insert("user_supplied_sample".to_string(), json!(manual));
        } else {
            context = json!({
                "auto_context": context,
                "user_supplied_sample": manual,
            });
        }
    }
    serde_json::to_string_pretty(&context).map_err(|e| e.to_string())
}
