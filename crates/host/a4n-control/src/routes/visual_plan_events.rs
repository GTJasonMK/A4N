use super::{auth::authenticated_user, error::ApiError};
use crate::state::ServerState;
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanView, ReadChapterVisualPlanInput, VisualPlanProjection, VisualPlanReadView,
};
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
};
use futures::stream;
use serde::Deserialize;
use std::{collections::VecDeque, convert::Infallible, time::Duration};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct QueryInput {
    project_id: i64,
    source_chapter_id: i64,
    #[serde(default)]
    after_sequence: i64,
    expected_source_hash: Option<String>,
    expected_record_id: Option<i64>,
}

async fn read(
    state: &ServerState,
    user_id: i64,
    input: &QueryInput,
) -> Result<Vec<ChapterVisualPlanView>, ApiError> {
    let result = a4n_runtime::host_api::chapter_visual_plan_records::read_chapter_visual_plan(
        state.runtime(),
        user_id,
        ReadChapterVisualPlanInput {
            project_id: input.project_id,
            source_chapter_id: input.source_chapter_id,
            record_id: None,
            expected_revision: None,
            view: VisualPlanReadView::Edits {
                after_sequence: input.after_sequence,
            },
        },
    )
    .await?;
    if input
        .expected_source_hash
        .as_ref()
        .is_some_and(|hash| hash != &result.source_hash)
    {
        return Err(ApiError::conflict(
            "编排来源已变化，请保留本地草稿并重新读取",
        ));
    }
    match result.projection {
        VisualPlanProjection::Edits { items, .. } => {
            if items.is_empty()
                && input.expected_record_id.is_some_and(|id| {
                    result
                        .resource
                        .as_ref()
                        .is_none_or(|resource| resource.id != id.to_string())
                })
            {
                return Err(ApiError::conflict("编排已删除、重建或切换版本，请重新读取"));
            }
            Ok(items)
        }
        _ => Err(ApiError::bad_request("编辑回执协议不兼容")),
    }
}

pub(super) async fn events(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(mut input): Query<QueryInput>,
) -> Result<Response, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    if let Some(value) = headers.get("last-event-id") {
        let cursor = value
            .to_str()
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value >= 0)
            .ok_or_else(|| ApiError::bad_request("Last-Event-ID 无效"))?;
        input.after_sequence = input.after_sequence.max(cursor);
    }
    let pending: VecDeque<_> = read(&state, user.id, &input).await?.into();
    let stream = stream::unfold(
        (state, user.id, input, pending),
        |(state, user_id, mut input, mut pending)| async move {
            loop {
                if let Some(item) = pending.pop_front() {
                    let Some(receipt) = &item.edit_receipt else {
                        tracing::error!("持久编辑回执缺少身份");
                        return None;
                    };
                    input.after_sequence = receipt.sequence;
                    input.expected_record_id = Some(receipt.after.record_id);
                    let event = Event::default()
                        .event("visual_plan_edit")
                        .id(receipt.sequence.to_string())
                        .data(serde_json::to_string(&item).expect("visual plan is JSON"));
                    return Some((Ok::<_, Infallible>(event), (state, user_id, input, pending)));
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                match read(&state, user_id, &input).await {
                    Ok(items) => pending.extend(items),
                    Err(error) => {
                        tracing::warn!(?error, "编辑回执续传失败，关闭流以重新校验权限和来源");
                        return None;
                    }
                }
            }
        },
    );
    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response())
}
