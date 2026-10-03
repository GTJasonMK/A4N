use serde_json::{Value, json};

use a4n_api_contract::optimize::OPTIMIZATION_CONTEXT_SOURCES;

pub(in crate::optimize) const GET_PROJECT_INFORMATION_TOOL: &str = "get_project_information";
pub(in crate::optimize) const EDIT_ARTICLE_TOOL: &str = "edit_article";

pub(crate) fn optimization_runtime_tool_specs() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": GET_PROJECT_INFORMATION_TOOL,
                "description": "读取或搜索当前优化会话可访问的项目、章节、章纲、蓝图、角色、伏笔和前后文章节信息。",
                "parameters": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["mode"],
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["read", "search"],
                            "description": "read 有界读取快照；current_chapter 返回带 ASCII 井号十进制标签的正文页和短 source_span_ids 列表，不返回内部 span ID。search 返回关键词片段，不能替代完整证据。"
                        },
                        "source": {
                            "type": "string",
                            "enum": OPTIMIZATION_CONTEXT_SOURCES,
                            "description": "read 模式必填；search 模式可选，不填时搜索全部上下文段。"
                        },
                        "query": {
                            "type": "string",
                            "description": "search 模式必填的关键词。"
                        },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 4096,
                            "description": "read 文本默认 2048/最多4096字符，数组或字段默认20/最多100项；search 默认8/最多20条。结果最多16KiB，超限需缩小范围。"
                        },
                        "offset": {
                            "type": "integer", "minimum": 0,
                            "description": "read 页偏移，按返回的 unit 计数；用 next_offset 继续读取。"
                        },
                        "path": {
                            "type": "string",
                            "description": "read 的 JSON pointer，如 /summary；省略则读来源根。current_chapter 的根路径或 /content 会返回带来源短标签的正文和 source_span_ids；禁止读取 /source_spans。"
                        },
                        "expected_hash": {
                            "type": "string",
                            "description": "read 后续页携带上页 resource.hash，快照变化时明确失败。"
                        }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": EDIT_ARTICLE_TOOL,
                "description": "提交当前章节的文章补丁，返回待用户审阅的补丁提案；真正写回正文只发生在用户接受并同步补丁之后。",
                "parameters": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["summary", "patches"],
                    "properties": {
                        "summary": {
                            "type": "string",
                            "description": "补丁提案摘要。"
                        },
                        "patches": {
                            "type": "array",
                            "description": "局部补丁列表。",
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "required": ["patch_id", "operation", "old_text", "new_text", "reason"],
                                "properties": {
                                    "patch_id": { "type": "string" },
                                    "issue_id": { "type": ["string", "null"] },
                                    "operation": {
                                        "type": "string",
                                        "enum": ["replace", "delete", "insert_before", "insert_after"],
                                        "description": "replace=修改；delete=删除 old_text；insert_before/insert_after=在唯一 old_text 锚点前/后新增 new_text。"
                                    },
                                    "old_text": {
                                        "type": "string",
                                        "description": "必须从当前章节正文逐字复制且唯一匹配；插入操作中它是定位锚点。"
                                    },
                                    "new_text": {
                                        "type": "string",
                                        "description": "replace/insert_* 的新文本；delete 时必须为空字符串；replace 时必须与 old_text 有实际差异。"
                                    },
                                    "reason": { "type": "string" },
                                    "risk": { "type": ["string", "null"] }
                                }
                            }
                        }
                    }
                }
            }
        }),
    ]
}

/// The chat loop only needs the callable shape in its stable prompt. Keep the
/// full schema as the execution contract and derive this smaller projection so
/// prompt size does not grow with descriptive text.
pub(crate) fn optimization_runtime_tool_specs_for_prompt() -> Vec<Value> {
    optimization_runtime_tool_specs()
        .into_iter()
        .filter_map(|spec| {
            let function = spec.get("function")?;
            Some(json!({
                "name": function.get("name"),
                "description": function.get("description"),
                "parameters": compact_schema(function.get("parameters")?),
            }))
        })
        .collect()
}

fn compact_schema(schema: &Value) -> Value {
    let Some(object) = schema.as_object() else {
        return Value::Null;
    };
    let mut compact = serde_json::Map::new();
    for key in ["type", "enum", "minimum", "maximum", "required"] {
        if let Some(value) = object.get(key) {
            compact.insert(key.to_string(), value.clone());
        }
    }
    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        compact.insert(
            "properties".to_string(),
            Value::Object(
                properties
                    .iter()
                    .map(|(name, value)| (name.clone(), compact_schema(value)))
                    .collect(),
            ),
        );
    }
    if let Some(items) = object.get("items") {
        compact.insert("items".to_string(), compact_schema(items));
    }
    Value::Object(compact)
}
