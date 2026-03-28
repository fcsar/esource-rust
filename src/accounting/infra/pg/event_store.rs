use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounting::application::traits::EventStore;
use crate::accounting::domain::errors::LedgerError;
use crate::accounting::domain::event::Event;
use crate::accounting::domain::types::{AccountId, Amount, EventId};

pub struct PgEventStore {
    pool: PgPool,
}

impl PgEventStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

struct EventRow {
    id: Uuid,
    account_id: Uuid,
    event_type: String,
    payload: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl EventRow {
    fn into_event(self) -> Result<Event, LedgerError> {
        match self.event_type.as_str() {
            "AccountCreated" => {
                let owner = self.payload["owner"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                Ok(Event::AccountCreated {
                    event_id: EventId(self.id),
                    account_id: AccountId(self.account_id),
                    owner,
                    created_at: self.created_at,
                })
            }
            "MoneyDeposited" => {
                let amount = self.payload["amount"].as_i64().unwrap_or(0);

                Ok(Event::MoneyDeposited {
                    event_id: EventId(self.id),
                    account_id: AccountId(self.account_id),
                    amount: Amount(amount),
                    created_at: self.created_at,
                })
            }
            "MoneyWithdrawn" => {
                let amount = self.payload["amount"].as_i64().unwrap_or(0);

                Ok(Event::MoneyWithdrawn {
                    event_id: EventId(self.id),
                    account_id: AccountId(self.account_id),
                    amount: Amount(amount),
                    created_at: self.created_at,
                })
            }
            _ => Err(LedgerError::UnknownEventType(self.event_type)),
        }
    }
}

fn event_to_type_and_payload(event: &Event) -> (&str, serde_json::Value) {
    match event {
        Event::AccountCreated { owner, .. } => {
            ("AccountCreated", serde_json::json!({ "owner": owner }))
        }
        Event::MoneyDeposited { amount, .. } => {
            ("MoneyDeposited", serde_json::json!({ "amount": amount.0 }))
        }
        Event::MoneyWithdrawn { amount, .. } => {
            ("MoneyWithdrawn", serde_json::json!({ "amount": amount.0 }))
        }
    }
}

fn event_id(event: &Event) -> Uuid {
    match event {
        Event::AccountCreated { event_id, .. }
        | Event::MoneyDeposited { event_id, .. }
        | Event::MoneyWithdrawn { event_id, .. } => event_id.0,
    }
}

fn event_account_id(event: &Event) -> Uuid {
    match event {
        Event::AccountCreated { account_id, .. }
        | Event::MoneyDeposited { account_id, .. }
        | Event::MoneyWithdrawn { account_id, .. } => account_id.0,
    }
}

fn event_created_at(event: &Event) -> DateTime<Utc> {
    match event {
        Event::AccountCreated { created_at, .. }
        | Event::MoneyDeposited { created_at, .. }
        | Event::MoneyWithdrawn { created_at, .. } => *created_at,
    }
}

impl EventStore for PgEventStore {
    async fn append(&self, event: &Event) -> Result<(), LedgerError> {
        let id = event_id(event);
        let account_id = event_account_id(event);
        let created_at = event_created_at(event);
        let (event_type, payload) = event_to_type_and_payload(event);

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO ledger_events (id, account_id, event_type, payload, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(account_id)
        .bind(event_type)
        .bind(&payload)
        .bind(created_at)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO event_outbox (id, event_id, status, created_at)
             VALUES ($1, $2, 'pending', $3)",
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(created_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    async fn load_events(&self, account_id: &AccountId) -> Result<Vec<Event>, LedgerError> {
        let rows = sqlx::query_as!(
            EventRow,
            "SELECT id, account_id, event_type, payload, created_at
             FROM ledger_events
             WHERE account_id = $1
             ORDER BY created_at",
            account_id.0
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| row.into_event()).collect()
    }
}
