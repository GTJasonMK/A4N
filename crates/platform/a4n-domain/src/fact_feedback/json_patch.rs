use super::{FactChangeOperation, FactMaterialChange};
use serde_json::Value;

pub fn apply_fact_json_change(
    document: &mut Value,
    change: &FactMaterialChange,
) -> Result<bool, String> {
    use FactChangeOperation::{Add, Remove, Replace};
    let current = document.pointer(&change.path);
    if change.operation == Add {
        if current.is_some() || !change.before.is_null() {
            return Err(format!("新增字段已经存在或 before 非空: {}", change.path));
        }
    } else if current != Some(&change.before) {
        return Err(format!("资料已变化，不能应用旧变更: {}", change.path));
    }
    if change.operation == Replace && change.before == change.after {
        return Ok(false);
    }
    if change.path.is_empty() {
        *document = if change.operation == Remove {
            Value::Null
        } else {
            change.after.clone()
        };
        return Ok(true);
    }
    let (parent, key) = change
        .path
        .rsplit_once('/')
        .ok_or("资料字段路径必须是 JSON Pointer")?;
    let key = decode_pointer_key(key)?;
    let parent = document
        .pointer_mut(parent)
        .ok_or("资料字段的父节点不存在")?;
    match parent {
        Value::Object(object) => {
            if change.operation == Remove {
                object.remove(&key);
            } else {
                object.insert(key, change.after.clone());
            }
        }
        Value::Array(items) => {
            if change.operation == Add && key == "-" {
                items.push(change.after.clone());
            } else {
                let index = key.parse::<usize>().map_err(|_| "数组字段索引无效")?;
                if index >= items.len() {
                    return Err("数组字段索引超出范围".into());
                }
                match change.operation {
                    Add => return Err("数组新增只支持 /- 追加".into()),
                    Remove => {
                        items.remove(index);
                    }
                    Replace => items[index] = change.after.clone(),
                }
            }
        }
        _ => return Err("资料字段父节点必须是对象或数组".into()),
    }
    Ok(true)
}

fn decode_pointer_key(key: &str) -> Result<String, String> {
    let mut decoded = String::new();
    let mut chars = key.chars();
    while let Some(character) = chars.next() {
        decoded.push(if character == '~' {
            match chars.next() {
                Some('0') => '~',
                Some('1') => '/',
                _ => return Err("资料字段路径包含无效转义".into()),
            }
        } else {
            character
        });
    }
    Ok(decoded)
}
