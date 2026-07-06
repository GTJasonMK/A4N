//! `<!-- @include path -->` 指令解析器
//!
//! 在提示词正文中递归展开 `<!-- @include relative/path.md -->`，
//! 支持以下安全特性：
//!
//! - 路径相对 `prompts/` 根目录
//! - 循环检测（同一文件不会展开两次）
//! - 最大深度限制（默认 10，防止递归爆炸）
//! - 路径越界保护（include 不能跳出 prompts_dir）
//! - 自动剥离被 include 文件的 frontmatter（只插入正文）

use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use a4n_core::error::AppError;

const DEFAULT_MAX_DEPTH: usize = 10;

/// `@include` 正则：匹配 `<!-- @include relative/path -->`，路径可被单/双引号包裹
fn include_re() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<!--\s*@include\s+(.+?)\s*-->").unwrap())
}

/// 从 markdown 文件全文中剥离 YAML frontmatter（开头 `---\n...\n---\n`），返回正文
fn strip_frontmatter(content: &str) -> &str {
    if !content.starts_with("---") {
        return content;
    }
    // 找第二个 "---" 行
    let after_first = &content[3..];
    if let Some(end) = after_first.find("\n---") {
        let body_start = 3 + end + 4; // 跳过 "\n---"
        // 跳过紧跟的换行
        let rest = &content[body_start..];
        if let Some(stripped) = rest.strip_prefix('\n') {
            stripped
        } else {
            rest
        }
    } else {
        content
    }
}

fn unquote(raw: &str) -> &str {
    let s = raw.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// 公开入口：展开 body 中所有 @include 指令
///
/// - `body`：当前 prompt 的正文（不含 frontmatter）
/// - `current_file`：当前 prompt 的绝对路径（仅用于报错链路）
/// - `prompts_dir`：prompts/ 根目录的绝对路径，include 路径相对它解析
pub fn resolve_includes(
    body: &str,
    current_file: &Path,
    prompts_dir: &Path,
) -> Result<String, AppError> {
    let prompts_root = prompts_dir.canonicalize().map_err(|e| {
        AppError::Internal(anyhow::anyhow!(
            "无法 canonicalize prompts_dir {prompts_dir:?}：{e}"
        ))
    })?;
    let current_canon = current_file.canonicalize().map_err(|e| {
        AppError::Internal(anyhow::anyhow!(
            "无法 canonicalize current_file {current_file:?}：{e}"
        ))
    })?;
    let stack = vec![current_canon];
    resolve_recursive(body, &prompts_root, &stack, 0)
}

fn resolve_recursive(
    body: &str,
    prompts_root: &Path,
    stack: &[PathBuf],
    depth: usize,
) -> Result<String, AppError> {
    if depth > DEFAULT_MAX_DEPTH {
        let chain = stack
            .iter()
            .filter_map(|p| p.strip_prefix(prompts_root).ok())
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(AppError::Internal(anyhow::anyhow!(
            "提示词 @include 深度超过限制 ({DEFAULT_MAX_DEPTH})：{chain}"
        )));
    }

    let re = include_re();
    let mut last_end = 0usize;
    let mut out = String::with_capacity(body.len());
    let mut error: Option<AppError> = None;

    for caps in re.captures_iter(body) {
        if error.is_some() {
            break;
        }
        let m = caps.get(0).unwrap();
        out.push_str(&body[last_end..m.start()]);
        last_end = m.end();

        let Some(rel_match) = caps.get(1) else {
            error = Some(AppError::Internal(anyhow::anyhow!(
                "提示词 @include 指令缺少路径"
            )));
            break;
        };
        let rel_raw = rel_match.as_str();
        let rel = unquote(rel_raw).trim();
        if rel.is_empty() {
            continue;
        }

        let include_path_raw = prompts_root.join(rel);
        let Ok(include_path) = include_path_raw.canonicalize() else {
            error = Some(AppError::Internal(anyhow::anyhow!(
                "提示词 @include 文件不存在：{rel}"
            )));
            break;
        };

        // 路径越界保护：必须仍在 prompts_root 之下
        if !include_path.starts_with(prompts_root) {
            error = Some(AppError::Internal(anyhow::anyhow!(
                "提示词 @include 路径越界：{rel}"
            )));
            break;
        }

        // 循环检测
        if stack.iter().any(|p| p == &include_path) {
            let chain = stack
                .iter()
                .chain(std::iter::once(&include_path))
                .filter_map(|p| p.strip_prefix(prompts_root).ok())
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            error = Some(AppError::Internal(anyhow::anyhow!(
                "提示词 @include 循环引用：{chain}"
            )));
            break;
        }

        let text = match fs::read_to_string(&include_path) {
            Ok(t) => t,
            Err(e) => {
                error = Some(AppError::Internal(anyhow::anyhow!(
                    "读取 @include 文件失败 {rel}：{e}"
                )));
                break;
            }
        };
        let inner_body = strip_frontmatter(&text);

        let mut new_stack = stack.to_vec();
        new_stack.push(include_path);
        let expanded = match resolve_recursive(inner_body, prompts_root, &new_stack, depth + 1) {
            Ok(s) => s,
            Err(e) => {
                error = Some(e);
                break;
            }
        };
        out.push_str(expanded.trim_end_matches('\n'));
    }

    if let Some(e) = error {
        return Err(e);
    }

    out.push_str(&body[last_end..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn strip_frontmatter_basic() {
        let s = "---\nname: x\n---\nhello";
        assert_eq!(strip_frontmatter(s), "hello");
    }

    #[test]
    fn strip_frontmatter_no_frontmatter() {
        assert_eq!(strip_frontmatter("hello"), "hello");
    }

    #[test]
    fn basic_include() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let main = root.join("main.md");
        let partial = root.join("_partials").join("p.md");
        write(&partial, "PARTIAL_BODY");
        write(&main, "BEFORE\n<!-- @include _partials/p.md -->\nAFTER");
        let body = fs::read_to_string(&main).unwrap();
        let out = resolve_includes(&body, &main, root).unwrap();
        assert!(out.contains("BEFORE"));
        assert!(out.contains("PARTIAL_BODY"));
        assert!(out.contains("AFTER"));
    }

    #[test]
    fn include_strips_inner_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let main = root.join("main.md");
        let p = root.join("_partials").join("p.md");
        write(&p, "---\nname: p\n---\nONLY_BODY");
        write(&main, "<!-- @include _partials/p.md -->");
        let out = resolve_includes(&fs::read_to_string(&main).unwrap(), &main, root).unwrap();
        assert!(out.contains("ONLY_BODY"));
        assert!(!out.contains("name: p"));
    }

    #[test]
    fn cycle_detected() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let a = root.join("a.md");
        let b = root.join("b.md");
        write(&a, "<!-- @include b.md -->");
        write(&b, "<!-- @include a.md -->");
        let res = resolve_includes(&fs::read_to_string(&a).unwrap(), &a, root);
        let err = res.unwrap_err();
        assert!(format!("{err}").contains("循环引用"));
    }

    #[test]
    fn path_outside_root_rejected() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("inner");
        fs::create_dir_all(&root).unwrap();
        // 创建一个 root 外的文件
        let outside = tmp.path().join("outside.md");
        write(&outside, "OUTSIDE");
        let main = root.join("main.md");
        write(&main, "<!-- @include ../outside.md -->");
        let res = resolve_includes(&fs::read_to_string(&main).unwrap(), &main, &root);
        assert!(res.is_err());
    }

    #[test]
    fn missing_file_errors() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let main = root.join("main.md");
        write(&main, "<!-- @include nope.md -->");
        let res = resolve_includes(&fs::read_to_string(&main).unwrap(), &main, root);
        assert!(res.is_err());
    }

    #[test]
    fn current_file_must_exist() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let missing = root.join("missing.md");
        let err = resolve_includes("body", &missing, root).unwrap_err();
        assert!(format!("{err}").contains("current_file"));
    }

    #[test]
    fn quoted_path_works() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let main = root.join("main.md");
        let p = root.join("_partials").join("q.md");
        write(&p, "QBODY");
        write(&main, r#"<!-- @include "_partials/q.md" -->"#);
        let out = resolve_includes(&fs::read_to_string(&main).unwrap(), &main, root).unwrap();
        assert!(out.contains("QBODY"));
    }
}
