use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{api::AppState, events::LoggedEvent};

#[derive(Deserialize)]
pub struct LogsQuery {
    before: Option<String>,
    limit: Option<usize>,
}

/// Returns recent logged events (system + internal) in reverse chronological order.
pub async fn list_logs(
    State(state): State<AppState>,
    Query(query): Query<LogsQuery>,
) -> Json<Vec<LoggedEvent>> {
    let limit = query.limit.unwrap_or(300).min(1000);
    let logs = if let Some(before) = query.before {
        state.event_bus.snapshot_before(&before, limit)
    } else {
        state.event_bus.snapshot(limit)
    };
    Json(logs)
}
