use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::{api::AppState, events::LoggedEvent};

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct LogsQuery {
    before: Option<String>,
    limit: Option<usize>,
}

/// Returns recent logged events (system + internal) in reverse chronological order.
#[utoipa::path(
    get,
    path = "/api/v1/logs",
    tag = "logs",
    params(LogsQuery),
    responses(
        (status = 200, description = "Recent events", body = [LoggedEvent]),
        (status = 401, description = "Authentication required")
    ),
    security(
        ("basic_auth" = [])
    )
)]
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
