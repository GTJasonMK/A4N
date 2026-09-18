use super::*;

pub async fn create_optimization_turn<C>(
    db: &C,
    input: OptimizationTurnCreateInput,
) -> Result<optimization_turn::Model, OptimizeDbError>
where
    C: ConnectionTrait,
{
    let now = chrono::Utc::now().naive_utc();
    optimization_turn::ActiveModel {
        session_id: Set(input.session_id),
        agent_turn_id: Set(input.agent_turn_id),
        user_text: Set(input.user_text),
        status: Set(input.status),
        permissions_snapshot: Set(input.permissions_snapshot),
        final_reply: Set(None),
        error: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(OptimizeDbError::Database)
}

pub async fn optimization_turns<C>(
    db: &C,
    session_id: i64,
) -> Result<Vec<optimization_turn::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_turn::Entity::find()
        .filter(optimization_turn::Column::SessionId.eq(session_id))
        .order_by_asc(optimization_turn::Column::CreatedAt)
        .order_by_asc(optimization_turn::Column::Id)
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_turn_by_agent_turn_id<C>(
    db: &C,
    agent_turn_id: i64,
) -> Result<Option<optimization_turn::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_turn::Entity::find()
        .filter(optimization_turn::Column::AgentTurnId.eq(agent_turn_id))
        .one(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_turn_by_id<C>(
    db: &C,
    turn_id: i64,
) -> Result<Option<optimization_turn::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_turn::Entity::find_by_id(turn_id)
        .one(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn update_optimization_turn<C>(
    db: &C,
    input: OptimizationTurnUpdateInput,
) -> Result<optimization_turn::Model, OptimizeDbError>
where
    C: ConnectionTrait,
{
    let mut active: optimization_turn::ActiveModel = input.turn.into();
    if let Some(status) = input.status {
        active.status = Set(status);
    }
    if let Some(final_reply) = input.final_reply {
        active.final_reply = Set(final_reply);
    }
    if let Some(error) = input.error {
        active.error = Set(error);
    }
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    active.update(db).await.map_err(OptimizeDbError::Database)
}

/// Atomically persist the formal user/assistant messages and complete one chat turn.
pub async fn complete_optimization_chat_turn(
    tx: &sea_orm::DatabaseTransaction,
    session_id: i64,
    turn_id: i64,
    user_text: &str,
    final_reply: &str,
) -> Result<optimization_turn::Model, OptimizeDbError> {
    let (session, turn) = locked_optimization_session_turn(tx, session_id, turn_id).await?;
    if turn.status != "running" {
        return Err(OptimizeDbError::InvalidArgument(format!(
            "优化 Turn 当前状态不允许完成: {}",
            turn.status
        )));
    }
    let user_text = user_text.trim();
    let final_reply = final_reply.trim();
    if user_text.is_empty() || final_reply.is_empty() {
        return Err(OptimizeDbError::InvalidArgument(
            "优化对话正式消息不能为空".to_string(),
        ));
    }
    if turn.user_text != user_text {
        return Err(OptimizeDbError::InvalidArgument(
            "优化 Turn 用户消息与完成请求不一致".to_string(),
        ));
    }

    insert_optimization_session_message(
        tx,
        OptimizationSessionMessageInsertInput {
            session_id,
            role: "user".to_string(),
            content: user_text.to_string(),
            tool_calls: None,
        },
    )
    .await?;
    insert_optimization_session_message(
        tx,
        OptimizationSessionMessageInsertInput {
            session_id,
            role: "assistant".to_string(),
            content: final_reply.to_string(),
            tool_calls: None,
        },
    )
    .await?;

    let mut turn_active: optimization_turn::ActiveModel = turn.into();
    turn_active.status = Set("completed".to_string());
    turn_active.final_reply = Set(Some(final_reply.to_string()));
    turn_active.error = Set(None);
    turn_active.updated_at = Set(chrono::Utc::now().naive_utc());
    let completed = turn_active.update(tx).await?;

    let mut session_active: optimization_session::ActiveModel = session.into();
    session_active.updated_at = Set(chrono::Utc::now().naive_utc());
    session_active.update(tx).await?;
    Ok(completed)
}

/// Atomically persist one analysis result and expose the completed turn as the formal result.
pub async fn complete_optimization_analysis_turn(
    tx: &sea_orm::DatabaseTransaction,
    session_id: i64,
    turn_id: i64,
    assistant_content: &str,
    canonical_issues: Value,
) -> Result<optimization_turn::Model, OptimizeDbError> {
    let (session, turn) = locked_optimization_session_turn(tx, session_id, turn_id).await?;
    if session.status != "analyzing" {
        return Err(OptimizeDbError::InvalidArgument(format!(
            "优化分析会话当前状态不允许完成: {}",
            session.status
        )));
    }
    ensure_running_turn(&turn)?;
    let assistant_content = assistant_content.trim();
    if assistant_content.is_empty() {
        return Err(OptimizeDbError::InvalidArgument(
            "优化分析正式回复不能为空".to_string(),
        ));
    }
    if !canonical_issues.is_array() {
        return Err(OptimizeDbError::InvalidArgument(
            "优化分析问题列表必须是数组".to_string(),
        ));
    }

    insert_optimization_session_message(
        tx,
        OptimizationSessionMessageInsertInput {
            session_id,
            role: "assistant".to_string(),
            content: assistant_content.to_string(),
            tool_calls: None,
        },
    )
    .await?;

    let completed = update_optimization_turn(
        tx,
        OptimizationTurnUpdateInput {
            turn,
            status: Some("completed".to_string()),
            final_reply: Some(Some(assistant_content.to_string())),
            error: Some(None),
        },
    )
    .await?;
    update_optimization_session(
        tx,
        OptimizationSessionUpdateInput {
            session,
            status: Some("waiting_user".to_string()),
            issues: Some(Some(canonical_issues)),
            agent_permissions: None,
            agent_instruction: None,
            fixes_applied: None,
            summary: None,
        },
    )
    .await?;
    Ok(completed)
}

/// Atomically persist one patch proposal, its review event and completed turn.
pub async fn complete_optimization_patch_turn(
    tx: &sea_orm::DatabaseTransaction,
    input: CompleteOptimizationPatchTurnInput,
) -> Result<CompleteOptimizationPatchTurnResult, OptimizeDbError> {
    let (session, turn) =
        locked_optimization_session_turn(tx, input.session_id, input.turn_id).await?;
    ensure_running_turn(&turn)?;
    if input.patch_set.session_id != session.id
        || input.patch_set.turn_id != turn.id
        || input.patch_set.chapter_id != session.chapter_id
    {
        return Err(OptimizeDbError::InvalidArgument(
            "优化补丁提案与会话/Turn 不一致".to_string(),
        ));
    }
    let assistant_content = required_completion_text(&input.assistant_content, "优化补丁正式回复")?;
    let tool_content = required_completion_text(&input.tool_content, "优化补丁工具结果")?;
    let final_reply = required_completion_text(&input.final_reply, "优化补丁 Turn 回复")?;
    let session_summary = required_completion_text(&input.session_summary, "优化补丁会话摘要")?;
    let tool_name = required_completion_text(&input.tool_name, "优化补丁工具名称")?;
    let patch_count = input.patch_set.patches.len();
    let proposal_summary = input.patch_set.summary.clone();

    insert_optimization_session_message(
        tx,
        OptimizationSessionMessageInsertInput {
            session_id: session.id,
            role: "assistant".to_string(),
            content: assistant_content,
            tool_calls: None,
        },
    )
    .await?;
    insert_optimization_session_message(
        tx,
        OptimizationSessionMessageInsertInput {
            session_id: session.id,
            role: "tool".to_string(),
            content: tool_content,
            tool_calls: input.tool_calls,
        },
    )
    .await?;
    let patch_set = create_optimization_patch_set(tx, input.patch_set).await?;
    let (event_kind, public_text, payload) = match patch_set.as_ref() {
        Some(patch_set) => (
            "tool_result",
            format!("已生成 {patch_count} 个局部补丁，等待审阅。"),
            serde_json::json!({
                "tool": tool_name,
                "patch_set_id": patch_set.id,
                "patch_count": patch_count,
                "summary": proposal_summary,
            }),
        ),
        None => (
            "assistant_update",
            "没有生成可安全唯一定位的局部补丁。".to_string(),
            serde_json::json!({
                "tool": tool_name,
                "patch_count": 0,
                "summary": proposal_summary,
            }),
        ),
    };
    append_optimization_turn_event(
        tx,
        OptimizationTurnEventAppendInput {
            turn_id: turn.id,
            kind: event_kind.to_string(),
            status: "completed".to_string(),
            public_text,
            payload: Some(payload),
        },
    )
    .await?;
    update_optimization_session(
        tx,
        OptimizationSessionUpdateInput {
            session,
            status: None,
            issues: None,
            agent_permissions: None,
            agent_instruction: None,
            fixes_applied: None,
            summary: Some(Some(session_summary)),
        },
    )
    .await?;
    let turn = update_optimization_turn(
        tx,
        OptimizationTurnUpdateInput {
            turn,
            status: Some("completed".to_string()),
            final_reply: Some(Some(final_reply)),
            error: Some(None),
        },
    )
    .await?;
    Ok(CompleteOptimizationPatchTurnResult { turn, patch_set })
}

async fn locked_optimization_session_turn(
    tx: &sea_orm::DatabaseTransaction,
    session_id: i64,
    turn_id: i64,
) -> Result<(optimization_session::Model, optimization_turn::Model), OptimizeDbError> {
    let session = optimization_session::Entity::find_by_id(session_id)
        .lock_exclusive()
        .one(tx)
        .await?
        .ok_or_else(|| OptimizeDbError::InvalidArgument("优化会话不存在".to_string()))?;
    let turn = optimization_turn::Entity::find_by_id(turn_id)
        .lock_exclusive()
        .one(tx)
        .await?
        .ok_or_else(|| OptimizeDbError::InvalidArgument("优化 Turn 不存在".to_string()))?;
    if turn.session_id != session.id {
        return Err(OptimizeDbError::InvalidArgument(
            "优化 Turn 不属于目标会话".to_string(),
        ));
    }
    Ok((session, turn))
}

fn ensure_running_turn(turn: &optimization_turn::Model) -> Result<(), OptimizeDbError> {
    if turn.status != "running" {
        return Err(OptimizeDbError::InvalidArgument(format!(
            "优化 Turn 当前状态不允许完成: {}",
            turn.status
        )));
    }
    Ok(())
}

fn required_completion_text(value: &str, label: &str) -> Result<String, OptimizeDbError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(OptimizeDbError::InvalidArgument(format!("{label}不能为空")));
    }
    Ok(value.to_string())
}

pub async fn optimization_turn_events<C>(
    db: &C,
    turn_id: i64,
) -> Result<Vec<optimization_turn_event::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_turn_event::Entity::find()
        .filter(optimization_turn_event::Column::TurnId.eq(turn_id))
        .order_by_asc(optimization_turn_event::Column::Seq)
        .order_by_asc(optimization_turn_event::Column::Id)
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_turn_events_for_turns<C>(
    db: &C,
    turn_ids: &[i64],
) -> Result<Vec<optimization_turn_event::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    if turn_ids.is_empty() {
        return Ok(Vec::new());
    }
    optimization_turn_event::Entity::find()
        .filter(optimization_turn_event::Column::TurnId.is_in(turn_ids.to_vec()))
        .order_by_asc(optimization_turn_event::Column::TurnId)
        .order_by_asc(optimization_turn_event::Column::Seq)
        .order_by_asc(optimization_turn_event::Column::Id)
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn append_optimization_turn_event<C>(
    db: &C,
    input: OptimizationTurnEventAppendInput,
) -> Result<optimization_turn_event::Model, OptimizeDbError>
where
    C: ConnectionTrait,
{
    let next_seq = optimization_turn_event::Entity::find()
        .filter(optimization_turn_event::Column::TurnId.eq(input.turn_id))
        .order_by_desc(optimization_turn_event::Column::Seq)
        .one(db)
        .await
        .map_err(OptimizeDbError::Database)?
        .map(|event| {
            event.seq.checked_add(1).ok_or_else(|| {
                OptimizeDbError::InvalidArgument("优化 turn event seq 超出 i32 范围".to_string())
            })
        })
        .transpose()?
        .unwrap_or(1);

    optimization_turn_event::ActiveModel {
        turn_id: Set(input.turn_id),
        seq: Set(next_seq),
        kind: Set(input.kind),
        status: Set(input.status),
        public_text: Set(input.public_text),
        payload: Set(input.payload),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(OptimizeDbError::Database)
}
