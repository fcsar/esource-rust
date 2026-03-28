use super::types::{AccountId, Amount};

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("account {0:?} not found")]
    AccountNotFound(AccountId),

    #[error("account {0:?} already exists")]
    DuplicateAccount(AccountId),

    #[error("insufficient funds: available {available:?}, requested {requested:?}")]
    InsufficientFunds { available: Amount, requested: Amount },

    #[error("invalid amount: must be greater than zero")]
    InvalidAmount,

    #[error("unknown event type: {0}")]
    UnknownEventType(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}
