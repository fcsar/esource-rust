use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

struct OutboxEntry {
    id: Uuid,
    event_id: Uuid,
}

struct EventRow {
    id: Uuid,
    account_id: Uuid,
    event_type: String,
    payload: serde_json::Value,
    created_at: DateTime<Utc>,
}

pub async fn run(pool: PgPool) {
    tracing::info!("projection worker started");

    loop {
        match process_pending(&pool).await {
            Ok(count) => {
                if count == 0 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                tracing::error!("projection worker error: {e}");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn process_pending(pool: &PgPool) -> Result<usize, sqlx::Error> {
    let entries = sqlx::query_as!(
        OutboxEntry,
        "SELECT id, event_id FROM event_outbox
         WHERE status = 'pending'
         ORDER BY created_at
         LIMIT 50"
    )
    .fetch_all(pool)
    .await?;

    let count = entries.len();

    for entry in entries {
        process_entry(pool, entry).await?;
    }

    Ok(count)
}

async fn process_entry(pool: &PgPool, entry: OutboxEntry) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE event_outbox SET status = 'processing' WHERE id = $1",
        entry.id
    )
    .execute(pool)
    .await?;

    let event = sqlx::query_as!(
        EventRow,
        "SELECT id, account_id, event_type, payload, created_at
         FROM ledger_events WHERE id = $1",
        entry.event_id
    )
    .fetch_one(pool)
    .await?;

    match event.event_type.as_str() {
        "AccountCreated" => {
            let owner = event.payload["owner"]
                .as_str()
                .unwrap_or_default();

            sqlx::query!(
                "INSERT INTO account_balances (account_id, owner, balance, last_event_id, updated_at)
                 VALUES ($1, $2, 0, $3, now())
                 ON CONFLICT (account_id) DO NOTHING",
                event.account_id,
                owner,
                event.id
            )
            .execute(pool)
            .await?;
        }
        "MoneyDeposited" => {
            let amount = event.payload["amount"].as_i64().unwrap_or(0);

            sqlx::query!(
                "UPDATE account_balances
                 SET balance = balance + $1, last_event_id = $2, updated_at = now()
                 WHERE account_id = $3 AND last_event_id != $2",
                amount,
                event.id,
                event.account_id
            )
            .execute(pool)
            .await?;

            sqlx::query!(
                "INSERT INTO transaction_history (id, account_id, event_type, amount, balance_after, created_at)
                 SELECT $1, $2, $3, $4,
                        (SELECT balance FROM account_balances WHERE account_id = $2),
                        $5
                 WHERE NOT EXISTS (SELECT 1 FROM transaction_history WHERE id = $1)",
                Uuid::new_v4(),
                event.account_id,
                &event.event_type,
                amount,
                event.created_at
            )
            .execute(pool)
            .await?;
        }
        "MoneyWithdrawn" => {
            let amount = event.payload["amount"].as_i64().unwrap_or(0);

            sqlx::query!(
                "UPDATE account_balances
                 SET balance = balance - $1, last_event_id = $2, updated_at = now()
                 WHERE account_id = $3 AND last_event_id != $2",
                amount,
                event.id,
                event.account_id
            )
            .execute(pool)
            .await?;

            sqlx::query!(
                "INSERT INTO transaction_history (id, account_id, event_type, amount, balance_after, created_at)
                 SELECT $1, $2, $3, $4,
                        (SELECT balance FROM account_balances WHERE account_id = $2),
                        $5
                 WHERE NOT EXISTS (SELECT 1 FROM transaction_history WHERE id = $1)",
                Uuid::new_v4(),
                event.account_id,
                &event.event_type,
                amount,
                event.created_at
            )
            .execute(pool)
            .await?;
        }
        other => {
            tracing::warn!("unknown event type: {other}");
        }
    }

    sqlx::query!(
        "UPDATE event_outbox SET status = 'done', processed_at = now() WHERE id = $1",
        entry.id
    )
    .execute(pool)
    .await?;

    tracing::debug!(event_id = %entry.event_id, "event projected");

    Ok(())
}
