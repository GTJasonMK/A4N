use serde_json::{Value, json};

pub(in crate::optimize) const GET_PROJECT_INFORMATION_TOOL: &str = "get_project_information";
pub(in crate::optimize) const EDIT_ARTICLE_TOOL: &str = "edit_article";

pub(in crate::optimize) const INFORMATION_SOURCES: &[&str] = &[
    "current_chapter",
    "current_chapter_outline",
    "project_detail",
    "blueprint",
    "part_outlines",
    "chapter_outlines",
    "character_profiles",
    "foreshadowing_index",
    "previous_chapters",
    "next_chapters",
];

pub(in crate::optimize) fn optimization_runtime_tool_specs() -> Vec<Value> {
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
                            "description": "read 按 source 读取完整上下文段；search 按 query 在指定 sources 或全部上下文中搜索。"
                        },
                        "source": {
                            "type": "string",
                            "enum": INFORMATION_SOURCES,
                            "description": "read 模式必填；search 模式可选，不填时搜索全部上下文段。"
                        },
                        "query": {
                            "type": "string",
                            "description": "search 模式必填的关键词。"
                        },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 20,
                            "description": "search 模式返回的最大匹配数；不填时返回 8 条。"
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
