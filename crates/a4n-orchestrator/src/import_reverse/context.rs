mod blueprint;
mod chapter_entries;
mod extraction;
mod project;
#[cfg(test)]
mod test_support;

use a4n_core::source_spans::source_text_hash;
use a4n_db::import_reverse::ReverseContextRows;
use serde_json::json;

use self::blueprint::has_blueprint_source;
use self::project::project_reverse_context;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReverseContextKind {
    Blueprint,
    Outline,
    PartOutline,
}

#[derive(Debug, Clone)]
pub(super) struct ReverseContentRequest<'a> {
    pub(super) project_id: i64,
    pub(super) manual_sample: &'a str,
    pub(super) auto_context: bool,
    pub(super) context_limit: Option<usize>,
}

fn required_reverse_context_limit(limit: Option<usize>) -> Result<usize, String> {
    match limit {
        Some(limit) if limit > 0 => Ok(limit),
        _ => Err("启用项目上下文时必须提供 context_limit 且大于 0".to_string()),
    }
}

pub(super) fn resolve_reverse_content_sample_with_provenance(
    input: ReverseContentRequest<'_>,
    kind: ReverseContextKind,
    rows: Option<&ReverseContextRows>,
) -> Result<(String, Option<String>), String> {
    let manual = input.manual_sample.trim();
    if !input.auto_context {
        if manual.is_empty() {
            return Err("请提供反推样本，或启用项目上下文".to_string());
        }
        return Ok((manual.to_string(), None));
    }

    let limit = required_reverse_context_limit(input.context_limit)?;
    let rows = rows.ok_or_else(|| "启用项目上下文时缺少反推上下文行".to_string())?;
    if kind == ReverseContextKind::Blueprint && manual.is_empty() && !has_blueprint_source(rows) {
        return Err(
            "当前项目还没有可用于归纳蓝图的正文、简介、大纲或正式资料；只有想法时请先使用灵感对话"
                .to_string(),
        );
    }
    let mut context = project_reverse_context(rows, input.project_id, kind, limit)?;
    let source_fingerprint =
        source_text_hash(&serde_json::to_string(&context).map_err(|error| error.to_string())?);
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
    serde_json::to_string_pretty(&context)
        .map(|content| (content, Some(source_fingerprint)))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::required_reverse_context_limit;

    #[test]
    fn context_limit_requires_explicit_value() {
        let err = required_reverse_context_limit(None).unwrap_err();

        assert!(err.contains("context_limit"));
    }

    #[test]
    fn context_limit_rejects_zero() {
        let err = required_reverse_context_limit(Some(0)).unwrap_err();

        assert!(err.contains("大于 0"));
    }

    #[test]
    fn context_limit_accepts_positive_value() {
        let limit = required_reverse_context_limit(Some(50)).unwrap();

        assert_eq!(limit, 50);
    }
}
