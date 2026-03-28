use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use uuid::Uuid;

use crate::accounting::application::query_handler::QueryHandler;
use crate::accounting::application::replay;
use crate::accounting::domain::errors::LedgerError;
use crate::accounting::domain::types::AccountId;

pub type QueryState = Arc<QueryHandler>;

pub async fn get_balance(
    State(query): State<QueryState>,
    Path(id): Path<Uuid>,
) -> Response {
    match query.get_balance(&AccountId(id)).await {
        Ok(balance) => (StatusCode::OK, Json(serde_json::to_value(balance).unwrap())).into_response(),
        Err(err) => error_to_response(err),
    }
}

pub async fn get_statement(
    State(query): State<QueryState>,
    Path(id): Path<Uuid>,
) -> Response {
    match query.get_statement(&AccountId(id)).await {
        Ok(records) => (StatusCode::OK, Json(serde_json::to_value(records).unwrap())).into_response(),
        Err(err) => error_to_response(err),
    }
}

pub async fn replay(State(query): State<QueryState>) -> Response {
    match replay::execute_replay(query.pool()).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::to_value(result).unwrap())).into_response(),
        Err(e) => {
            tracing::error!("replay failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "replay failed" })),
            )
                .into_response()
        }
    }
}

fn error_to_response(err: LedgerError) -> Response {
    let (status, message) = match &err {
        LedgerError::AccountNotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".to_string()),
    };

    (status, Json(serde_json::json!({ "error": message }))).into_response()
}
