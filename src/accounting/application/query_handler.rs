use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounting::domain::errors::LedgerError;
use crate::accounting::domain::types::AccountId;

#[derive(Debug, Serialize)]
pub struct AccountBalance {
    pub account_id: Uuid,
    pub owner: String,
    pub balance: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TransactionRecord {
    pub id: Uuid,
    pub account_id: Uuid,
    pub event_type: String,
    pub amount: i64,
    pub balance_after: i64,
    pub created_at: DateTime<Utc>,
}

pub struct QueryHandler {
    pool: PgPool,
}

impl QueryHandler {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn get_balance(&self, account_id: &AccountId) -> Result<AccountBalance, LedgerError> {
        let row = sqlx::query_as!(
            AccountBalance,
            "SELECT account_id, owner, balance, updated_at
             FROM account_balances
             WHERE account_id = $1",
            account_id.0
        )
        .fetch_optional(&self.pool)
        .await?;

        row.ok_or_else(|| LedgerError::AccountNotFound(account_id.clone()))
    }

    pub async fn get_statement(&self, account_id: &AccountId) -> Result<Vec<TransactionRecord>, LedgerError> {
        let rows = sqlx::query_as!(
            TransactionRecord,
            "SELECT id, account_id, event_type, amount, balance_after, created_at
             FROM transaction_history
             WHERE account_id = $1
             ORDER BY created_at DESC",
            account_id.0
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
