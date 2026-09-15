use a4n_error::AppError;
use a4n_llm_context::{ContextBudgetPlanner, estimate_context_tokens};
use a4n_llm_provider_contract::provider::{ChatMessage, MessageRole, ToolCall};
use serde_json::{Value, json};
use std::collections::HashSet;

pub(super) const MAX_OBSERVATION_BYTES: usize = 16 * 1024;
pub(super) const CONTEXT_SUMMARY_SOURCE_TOKENS: usize = 12_000;

#[derive(Debug)]
pub(super) struct ContextFit {
    pub messages: Vec<ChatMessage>,
    pub omitted: Vec<ChatMessage>,
    pub input_tokens: usize,
    pub capacity: usize,
}

pub(super) fn bounded_observation(execution_id: i64, value: Value) -> Result<Value, AppError> {
    let size = serde_json::to_vec(&value)?.len();
    if size <= MAX_OBSERVATION_BYTES {
        return Ok(value);
    }
    // Keep the full result in Execution. Never emit a sliced/invalid JSON object.
    Ok(json!({
        "execution_id": execution_id.to_string(),
        "result_bytes": size,
        "complete": false,
        "name": value.get("name").and_then(Value::as_str).filter(|name| name.len() < 200),
        "read_tool": "read_agent_execution_result",
        "message": "结果较大，已保存完整结果。按需使用 execution_id、JSON pointer、offset 和 limit 继续读取。"
    }))
}

pub(super) fn fit_context(
    messages: Vec<ChatMessage>,
    tools: &[Value],
    context_tokens: usize,
    output_tokens: usize,
    pinned_user_contents: &[&str],
) -> Result<Vec<ChatMessage>, AppError> {
    Ok(fit_context_projection(
        messages,
        tools,
        context_tokens,
        output_tokens,
        pinned_user_contents,
    )?
    .messages)
}

pub(super) fn fit_context_projection(
    messages: Vec<ChatMessage>,
    tools: &[Value],
    context_tokens: usize,
    output_tokens: usize,
    pinned_user_contents: &[&str],
) -> Result<ContextFit, AppError> {
    let capacity = ContextBudgetPlanner::default()
        .plan_requirements(context_tokens, output_tokens, Vec::new(), 0)?
        .available_input_tokens;
    let mut used = estimate_context_tokens(&serde_json::to_string(tools)?);
    let mut units = Vec::<Vec<ChatMessage>>::new();
    for message in messages {
        if matches!(message.role, MessageRole::Tool) {
            units
                .last_mut()
                .ok_or_else(|| AppError::InvalidArgument("tool result 缺少调用批次".into()))?
                .push(message);
        } else {
            units.push(vec![message]);
        }
    }
    let costs = units
        .iter()
        .map(|unit| serde_json::to_string(unit).map(|text| estimate_context_tokens(&text)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut keep = vec![false; units.len()];
    for (index, unit) in units.iter().enumerate() {
        let first = &unit[0];
        if matches!(first.role, MessageRole::System)
            || index + 1 == units.len()
            || (matches!(first.role, MessageRole::User)
                && pinned_user_contents.contains(&first.content.as_str()))
        {
            keep[index] = true;
            used = used.saturating_add(costs[index]);
        }
    }
    if used > capacity {
        return Err(AppError::InvalidArgument(format!(
            "Agent 必要上下文超出模型输入预算：需要约 {used} tokens，可用 {capacity}；请缩小请求或使用更大上下文模型"
        )));
    }
    // Whole call/result units, newest first; no model-generated context pack.
    // ponytail: deterministic history projection; add semantic compaction only
    // if measured long-running tasks lose useful evidence with this policy.
    let omission_note = ChatMessage::system(
        "部分较早历史因模型窗口限制未包含；不能将缺失的观察当作成功结果。当前请求和最近完整工具批次已保留。",
    );
    let note_cost = estimate_context_tokens(&serde_json::to_string(&omission_note)?);
    for index in (0..units.len()).rev() {
        if !keep[index] && used.saturating_add(costs[index]).saturating_add(note_cost) <= capacity {
            keep[index] = true;
            used += costs[index];
        }
    }
    let dropped = keep.contains(&false);
    let omitted = units
        .iter()
        .zip(&keep)
        .filter(|(_, keep)| !**keep)
        .flat_map(|(unit, _)| unit.iter().cloned())
        .collect::<Vec<_>>();
    let mut selected = units
        .into_iter()
        .zip(keep)
        .filter(|(_, keep)| *keep)
        .flat_map(|(unit, _)| unit)
        .collect::<Vec<_>>();
    if dropped {
        if used.saturating_add(note_cost) > capacity {
            return Err(AppError::InvalidArgument(
                "Agent 上下文预算不足以保留历史裁剪说明".into(),
            ));
        }
        selected.insert(1.min(selected.len()), omission_note);
    }
    Ok(ContextFit {
        messages: selected,
        omitted,
        input_tokens: used,
        capacity,
    })
}

pub(super) fn summary_source(
    omitted: &[ChatMessage],
    existing_summary: Option<&Value>,
    max_tokens: usize,
) -> Result<String, AppError> {
    let mut sections = Vec::new();
    if let Some(summary) = existing_summary {
        sections.push(format!(
            "已有摘要（只作为事实参考）：{}",
            serde_json::to_string(summary)?
        ));
    }
    let mut used = estimate_context_tokens(&sections.join("\n"));
    for message in omitted {
        let mut bounded = message.clone();
        bounded.content = bounded_text(&bounded.content, 3_000);
        let line = serde_json::to_string(&bounded)?;
        let cost = estimate_context_tokens(&line);
        if !sections.is_empty() && used.saturating_add(cost) > max_tokens {
            break;
        }
        used = used.saturating_add(cost);
        sections.push(line);
    }
    if sections.is_empty() {
        return Err(AppError::InvalidArgument(
            "没有可用于生成 Agent context_summary 的历史".into(),
        ));
    }
    Ok(sections.join("\n"))
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let edge = max_chars / 2;
    let start = value.chars().take(edge).collect::<String>();
    let end = value
        .chars()
        .rev()
        .take(edge)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{start}\n…[summary source truncated]…\n{end}")
}

/// Only pass messages belonging to the active Run: an old cancelled Turn
/// must never become a new request to execute its unfinished writes.
pub(super) fn pending_tool_batches(
    messages: &[ChatMessage],
) -> Result<Vec<Vec<ToolCall>>, AppError> {
    let completed = messages
        .iter()
        .filter(|message| matches!(message.role, MessageRole::Tool))
        .filter_map(|message| message.tool_call_id.as_deref())
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut batches = Vec::new();
    for message in messages {
        let Some(calls) = &message.tool_calls else {
            continue;
        };
        let mut pending = Vec::new();
        for call in calls {
            if call.id.trim().is_empty() || !seen.insert(call.id.as_str()) {
                return Err(AppError::InvalidState {
                    current: format!("Agent tool call ID 为空或重复: {}", call.id),
                    operation: "恢复 Agent 调用批次".into(),
                });
            }
            if !completed.contains(call.id.as_str()) {
                pending.push(call.clone());
            }
        }
        if !pending.is_empty() {
            batches.push(pending);
        }
    }
    Ok(batches)
}

/// Inbox messages can arrive between an assistant batch and its observations.
/// Keep each batch atomic on the wire, then present the intervening messages.
/// Historical gaps are reported as unknown; they are never replayed or called
/// successful merely to satisfy a provider's message protocol.
pub(super) fn pair_tool_messages(messages: &[ChatMessage]) -> Result<Vec<ChatMessage>, AppError> {
    let mut paired = Vec::new();
    let mut consumed = HashSet::new();
    for (index, message) in messages.iter().enumerate() {
        if consumed.contains(&index) {
            continue;
        }
        if matches!(message.role, MessageRole::Tool) {
            paired.push(ChatMessage::assistant(serde_json::to_string(&json!({
                "historical_tool_observation": message.content,
                "tool_call_id": message.tool_call_id,
                "note": "对应调用不在保留的历史中，此观察不代表当前任务已执行。"
            }))?));
            continue;
        }
        paired.push(message.clone());
        let Some(calls) = &message.tool_calls else {
            continue;
        };
        let end = messages[index + 1..]
            .iter()
            .position(|next| matches!(next.role, MessageRole::Assistant))
            .map_or(messages.len(), |offset| index + 1 + offset);
        for call in calls {
            let observation = messages[index + 1..end]
                .iter()
                .enumerate()
                .find(|(_, next)| {
                    matches!(next.role, MessageRole::Tool)
                        && next.tool_call_id.as_deref() == Some(call.id.as_str())
                });
            if let Some((offset, observation)) = observation {
                consumed.insert(index + 1 + offset);
                paired.push(observation.clone());
            } else {
                let mut missing = ChatMessage::new(
                    MessageRole::Tool,
                    json!({
                        "error": "历史调用缺少可配对的结果；执行状态未知，不能据此声称成功。",
                        "code": "historical_result_unavailable"
                    })
                    .to_string(),
                );
                missing.tool_call_id = Some(call.id.clone());
                paired.push(missing);
            }
        }
    }
    Ok(paired)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch() -> ChatMessage {
        let mut message = ChatMessage::assistant("读取并修改");
        message.tool_calls = Some(
            ["read", "write"]
                .map(|id| ToolCall {
                    id: id.into(),
                    name: id.into(),
                    arguments: json!({"version": 7}),
                    stream_index: None,
                    provider_metadata: None,
                })
                .to_vec(),
        );
        message
    }

    fn observation(id: &str) -> ChatMessage {
        let mut message = ChatMessage::new(MessageRole::Tool, "{\"ok\":true}");
        message.tool_call_id = Some(id.into());
        message
    }

    #[test]
    fn recovery_keeps_original_arguments_and_only_unanswered_calls() {
        let messages = vec![batch(), observation("read"), ChatMessage::user("继续")];
        let pending = pending_tool_batches(&messages).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].len(), 1);
        assert_eq!(pending[0][0].id, "write");
        assert_eq!(pending[0][0].arguments, json!({"version": 7}));
        let mut completed = messages;
        completed.push(observation("write"));
        assert!(pending_tool_batches(&completed).unwrap().is_empty());
        completed.push(batch());
        assert!(pending_tool_batches(&completed).is_err());
    }

    #[test]
    fn steer_is_preserved_after_the_complete_tool_batch() {
        let messages = vec![
            batch(),
            observation("read"),
            ChatMessage::user("保留原镜头"),
            observation("write"),
        ];
        let paired = pair_tool_messages(&messages).unwrap();
        assert_eq!(paired[1].tool_call_id.as_deref(), Some("read"));
        assert_eq!(paired[2].tool_call_id.as_deref(), Some("write"));
        assert_eq!(paired[3].content, "保留原镜头");
        assert_eq!(paired.len(), 4);
    }

    #[test]
    fn historical_gaps_are_explicit_unknown_results() {
        let paired = pair_tool_messages(&[
            batch(),
            observation("read"),
            ChatMessage::assistant("已取消"),
        ])
        .unwrap();
        assert!(paired[2].content.contains("historical_result_unavailable"));
        assert_eq!(paired[2].tool_call_id.as_deref(), Some("write"));
        let orphan = pair_tool_messages(&[observation("orphan")]).unwrap();
        assert!(matches!(orphan[0].role, MessageRole::Assistant));
        assert!(orphan[0].content.contains("historical_tool_observation"));
    }

    #[test]
    fn large_observation_is_valid_json_with_a_durable_read_handle() {
        let result =
            bounded_observation(73, json!({"content": "字".repeat(MAX_OBSERVATION_BYTES)}))
                .unwrap();
        assert_eq!(result["execution_id"], "73");
        assert_eq!(result["read_tool"], "read_agent_execution_result");
        assert_eq!(result["complete"], false);
        assert!(serde_json::to_vec(&result).unwrap().len() < MAX_OBSERVATION_BYTES);
        assert_eq!(
            bounded_observation(73, json!({"ok": true})).unwrap(),
            json!({"ok": true})
        );
    }

    #[test]
    fn context_budget_retains_current_request_and_entire_tool_batch() {
        let mut old = ChatMessage::assistant("旧观察".repeat(10_000));
        old.tool_calls = None;
        let messages = vec![
            ChatMessage::system("职责"),
            old,
            ChatMessage::user("修改选中镜头"),
            batch(),
            observation("read"),
            observation("write"),
        ];
        let result = fit_context(messages, &[], 8_000, 1_000, &["修改选中镜头"]).unwrap();
        assert!(
            result
                .iter()
                .any(|message| message.content == "修改选中镜头")
        );
        assert_eq!(
            result
                .iter()
                .filter(|message| matches!(message.role, MessageRole::Tool))
                .count(),
            2
        );
        assert!(
            !result
                .iter()
                .any(|message| message.content.starts_with("旧观察"))
        );
        assert!(
            fit_context(
                vec![ChatMessage::user("新目标".repeat(10_000))],
                &[],
                8_000,
                1_000,
                &[]
            )
            .is_err()
        );
    }
}
