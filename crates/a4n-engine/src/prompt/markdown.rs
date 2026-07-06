//! Markdown prompt frontmatter parsing.

use std::fs;
use std::path::Path;

use a4n_core::error::AppError;

/// Markdown file parsing result.
pub(super) struct ParsedPrompt {
    pub(super) name: String,
    pub(super) content: String,
}

/// Parse one Markdown prompt file into local frontmatter metadata and template content.
pub(super) fn parse_markdown_file(path: &Path) -> Result<ParsedPrompt, AppError> {
    let content = fs::read_to_string(path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("读取文件失败 {path:?}: {e}")))?;

    let (frontmatter, template) = split_frontmatter(&content)?;
    let name = parse_frontmatter(&frontmatter)?;

    Ok(ParsedPrompt {
        name,
        content: template,
    })
}

/// Split frontmatter and template content.
fn split_frontmatter(content: &str) -> Result<(String, String), AppError> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(AppError::Internal(anyhow::anyhow!(
            "缺少 frontmatter 分隔符"
        )));
    }

    let after_first = &content[3..];
    let end = after_first
        .find("\n---")
        .or_else(|| after_first.find("\r\n---"))
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("frontmatter 未闭合")))?;

    let frontmatter = after_first[..end].trim().to_string();
    let template = after_first[end + 4..].trim().to_string();

    if template.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!("模板内容为空")));
    }

    Ok((frontmatter, template))
}

/// Parse frontmatter fields owned by the Markdown file.
fn parse_frontmatter(raw: &str) -> Result<String, AppError> {
    extract_field(raw, "name").ok_or_else(|| AppError::Internal(anyhow::anyhow!("缺少 name 字段")))
}

/// Extract a scalar frontmatter field.
fn extract_field(raw: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&prefix) {
            let value = trimmed[prefix.len()..].trim();
            if value.is_empty() {
                return None;
            }
            let value = match value.split_once("//") {
                Some((head, _)) => head.trim(),
                None => value.trim(),
            };
            let value = value.trim_matches('"').trim_matches('\'');
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_scalar_and_list_frontmatter_fields() {
        let raw = "name: test\ncategory: outline\ndependencies:\n  - dep1\n  - dep2";

        assert_eq!(extract_field(raw, "name").as_deref(), Some("test"));
        assert_eq!(extract_field(raw, "category").as_deref(), Some("outline"));
    }

    #[test]
    fn splits_frontmatter_from_template() {
        let md = "---\nname: test\ntitle: 测试\n---\n你好，{{ name }}！";
        let (frontmatter, template) = split_frontmatter(md).expect("frontmatter");

        assert!(frontmatter.contains("name: test"));
        assert_eq!(template, "你好，{{ name }}！");
    }

    #[test]
    fn parses_markdown_prompt_name_and_content() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.md");
        let md = "---\nname: greeting\ntitle: 问候\ncategory: writing\ndescription: 一个问候模板\nversion: \"1.2\"\ndependencies:\n  - base\n---\n你好，{{ name }}！欢迎。";
        fs::write(&path, md).unwrap();

        let parsed = parse_markdown_file(&path).expect("parsed prompt");
        assert_eq!(parsed.name, "greeting");
        assert_eq!(parsed.content, "你好，{{ name }}！欢迎。");
    }

    #[test]
    fn rejects_missing_name() {
        let raw = "title: test";

        assert!(parse_frontmatter(raw).is_err());
    }
}
