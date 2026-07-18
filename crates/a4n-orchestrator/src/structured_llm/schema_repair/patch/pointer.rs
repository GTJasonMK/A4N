use a4n_core::error::AppError;
use serde_json::Value;

pub(super) fn object_parent_mut<'a>(
    candidate: &'a mut Value,
    path: &str,
) -> Result<(&'a mut serde_json::Map<String, Value>, String), AppError> {
    let index = path.rfind('/').ok_or_else(|| {
        AppError::InvalidArgument(format!("字段补丁路径不是合法 JSON Pointer：{path}"))
    })?;
    let parent_path = &path[..index];
    let key = decode_json_pointer_token(&path[index + 1..])?;
    let parent = candidate
        .pointer_mut(parent_path)
        .ok_or_else(|| AppError::InvalidArgument(format!("字段补丁父路径不存在：{parent_path}")))?;
    let object = parent.as_object_mut().ok_or_else(|| {
        AppError::InvalidArgument(format!("字段补丁父路径必须是对象：{parent_path}"))
    })?;
    Ok((object, key))
}

fn decode_json_pointer_token(token: &str) -> Result<String, AppError> {
    let mut output = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(ch) = chars.next() {
        if ch != '~' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('0') => output.push('~'),
            Some('1') => output.push('/'),
            _ => {
                return Err(AppError::InvalidArgument(format!(
                    "字段补丁包含非法 JSON Pointer 转义：{token}"
                )));
            }
        }
    }
    Ok(output)
}
