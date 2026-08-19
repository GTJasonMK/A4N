use std::fs;
use std::path::{Path, PathBuf};

use a4n_core::error::AppError;

use super::frontmatter::strip_frontmatter;
use super::syntax::{include_re, unquote};

const DEFAULT_MAX_DEPTH: usize = 10;

/// 展开 body 中的所有 `@include` 指令。
///
/// `current_file` 和 `prompts_dir` 必须是可 canonicalize 的真实路径；include 路径始终
/// 相对 canonical prompts 根目录解析。
pub(crate) fn resolve_includes(
    body: &str,
    current_file: &Path,
    prompts_dir: &Path,
) -> Result<String, AppError> {
    let prompts_root = prompts_dir.canonicalize().map_err(|error| {
        AppError::Internal(anyhow::anyhow!(
            "无法 canonicalize prompts_dir {prompts_dir:?}：{error}"
        ))
    })?;
    let current_canon = current_file.canonicalize().map_err(|error| {
        AppError::Internal(anyhow::anyhow!(
            "无法 canonicalize current_file {current_file:?}：{error}"
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
            .filter_map(|path| path.strip_prefix(prompts_root).ok())
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(AppError::Internal(anyhow::anyhow!(
            "提示词 @include 深度超过限制 ({DEFAULT_MAX_DEPTH})：{chain}"
        )));
    }

    let mut last_end = 0usize;
    let mut output = String::with_capacity(body.len());
    let mut error: Option<AppError> = None;

    for captures in include_re().captures_iter(body) {
        if error.is_some() {
            break;
        }
        let matched = captures.get(0).unwrap();
        output.push_str(&body[last_end..matched.start()]);
        last_end = matched.end();

        let Some(relative_match) = captures.get(1) else {
            error = Some(AppError::Internal(anyhow::anyhow!(
                "提示词 @include 指令缺少路径"
            )));
            break;
        };
        let relative = unquote(relative_match.as_str()).trim();
        if relative.is_empty() {
            continue;
        }

        let include_path_raw = prompts_root.join(relative);
        let Ok(include_path) = include_path_raw.canonicalize() else {
            error = Some(AppError::Internal(anyhow::anyhow!(
                "提示词 @include 文件不存在：{relative}"
            )));
            break;
        };

        if !include_path.starts_with(prompts_root) {
            error = Some(AppError::Internal(anyhow::anyhow!(
                "提示词 @include 路径越界：{relative}"
            )));
            break;
        }

        if stack.iter().any(|path| path == &include_path) {
            let chain = stack
                .iter()
                .chain(std::iter::once(&include_path))
                .filter_map(|path| path.strip_prefix(prompts_root).ok())
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            error = Some(AppError::Internal(anyhow::anyhow!(
                "提示词 @include 循环引用：{chain}"
            )));
            break;
        }

        let text = match fs::read_to_string(&include_path) {
            Ok(text) => text,
            Err(read_error) => {
                error = Some(AppError::Internal(anyhow::anyhow!(
                    "读取 @include 文件失败 {relative}：{read_error}"
                )));
                break;
            }
        };
        let inner_body = strip_frontmatter(&text);

        let mut new_stack = stack.to_vec();
        new_stack.push(include_path);
        let expanded = match resolve_recursive(inner_body, prompts_root, &new_stack, depth + 1) {
            Ok(expanded) => expanded,
            Err(resolve_error) => {
                error = Some(resolve_error);
                break;
            }
        };
        output.push_str(expanded.trim_end_matches('\n'));
    }

    if let Some(error) = error {
        return Err(error);
    }

    output.push_str(&body[last_end..]);
    Ok(output)
}
