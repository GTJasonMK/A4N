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
