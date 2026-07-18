use super::auth::authenticated_user_from_access_token;
use super::error::ApiError;
use crate::state::ServerState;
use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::{Stream, StreamExt, stream};
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Debug, Deserialize)]
pub(super) struct SubscribeQuery {
    access_token: String,
}

pub(super) async fn subscribe(
    State(state): State<ServerState>,
    Path(channel): Path<String>,
    Query(query): Query<SubscribeQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    authenticated_user_from_access_token(&state, &query.access_token).await?;
    let receiver = state.web_events.subscribe(&channel).await;
    let initial = stream::once(async { Ok(Event::default().comment("connected")) });
    let events = stream::unfold(receiver, |mut receiver| async move {
        loop {
            match receiver.recv().await {
                Ok(payload) => {
                    return Some((Ok(Event::default().data(payload.to_string())), receiver));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    let stream = initial.chain(events);

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(5))
            .text("keep-alive"),
    ))
}
