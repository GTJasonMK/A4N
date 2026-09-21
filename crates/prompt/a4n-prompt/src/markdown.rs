//! Markdown prompt frontmatter parsing.

use std::fs;
use std::path::Path;

use a4n_error::AppError;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptFrontmatter {
    name: String,
}

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
    let frontmatter: PromptFrontmatter = serde_yaml::from_str(raw)
        .map_err(|error| AppError::ConfigError(format!("提示词 frontmatter 无效：{error}")))?;
    if frontmatter.name.is_empty()
        || frontmatter.name.trim() != frontmatter.name
        || frontmatter.name.chars().any(char::is_control)
    {
        return Err(AppError::ConfigError(
            "提示词 frontmatter name 必须是无首尾空白的非空文本".to_string(),
        ));
    }
    Ok(frontmatter.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_accepts_only_the_local_name() {
        assert_eq!(parse_frontmatter("name: test").unwrap(), "test");
        assert!(parse_frontmatter("name: test\ntitle: legacy").is_err());
    }

    #[test]
    fn splits_frontmatter_from_template() {
        let md = "---\nname: test\n---\n你好，{{ name }}！";
        let (frontmatter, template) = split_frontmatter(md).expect("frontmatter");

        assert_eq!(frontmatter, "name: test");
        assert_eq!(template, "你好，{{ name }}！");
    }

    #[test]
    fn parses_markdown_prompt_name_and_content() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.md");
        let md = "---\nname: greeting\n---\n你好，{{ name }}！欢迎。";
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
