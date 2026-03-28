use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use uuid::Uuid;

use crate::accounting::application::command_handler::CommandHandler;
use crate::accounting::application::traits::EventStore;
use crate::accounting::domain::command::Command;
use crate::accounting::domain::errors::LedgerError;
use crate::accounting::domain::types::{AccountId, Amount};

pub type AppState<S> = Arc<CommandHandler<S>>;

#[derive(Deserialize)]
pub struct CreateAccountBody {
    pub owner: String,
}

#[derive(Deserialize)]
pub struct DepositBody {
    pub amount: i64,
}

#[derive(Deserialize)]
pub struct WithdrawBody {
    pub amount: i64,
}

pub async fn create_account<S: EventStore>(
    State(handler): State<AppState<S>>,
    Json(body): Json<CreateAccountBody>,
) -> Response {
    let cmd = Command::CreateAccount { owner: body.owner };

    match handler.handle(cmd).await {
        Ok(event) => (
            StatusCode::CREATED,
            Json(serde_json::to_value(event).unwrap()),
        )
            .into_response(),
        Err(err) => error_to_response(err),
    }
}

pub async fn deposit<S: EventStore>(
    State(handler): State<AppState<S>>,
    Path(id): Path<Uuid>,
    Json(body): Json<DepositBody>,
) -> Response {
    let cmd = Command::DepositMoney {
        account_id: AccountId(id),
        amount: Amount(body.amount),
    };

    match handler.handle(cmd).await {
        Ok(event) => (StatusCode::OK, Json(serde_json::to_value(event).unwrap())).into_response(),
        Err(err) => error_to_response(err),
    }
}

pub async fn withdraw<S: EventStore>(
    State(handler): State<AppState<S>>,
    Path(id): Path<Uuid>,
    Json(body): Json<WithdrawBody>,
) -> Response {
    let cmd = Command::WithdrawMoney {
        account_id: AccountId(id),
        amount: Amount(body.amount),
    };

    match handler.handle(cmd).await {
        Ok(event) => (StatusCode::OK, Json(serde_json::to_value(event).unwrap())).into_response(),
        Err(err) => error_to_response(err),
    }
}

fn error_to_response(err: LedgerError) -> Response {
    let (status, message) = match &err {
        LedgerError::AccountNotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
        LedgerError::DuplicateAccount(_) => (StatusCode::CONFLICT, err.to_string()),
        LedgerError::InsufficientFunds { .. } => {
            (StatusCode::UNPROCESSABLE_ENTITY, err.to_string())
        }
        LedgerError::InvalidAmount => (StatusCode::BAD_REQUEST, err.to_string()),
        LedgerError::UnknownEventType(_) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        LedgerError::Database(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error".to_string(),
        ),
    };

    (status, Json(serde_json::json!({ "error": message }))).into_response()
}
