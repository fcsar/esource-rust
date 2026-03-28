use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct ReplayResult {
    pub events_to_process: i64,
}

pub async fn execute_replay(pool: &PgPool) -> Result<ReplayResult, sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM transaction_history")
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM account_balances")
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE event_outbox SET status = 'pending', processed_at = NULL")
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM event_outbox")
        .fetch_one(pool)
        .await?;

    tracing::info!(events = count, "replay started");

    Ok(ReplayResult {
        events_to_process: count,
    })
}
