# edriven

Financial Ledger built with CQRS + Event Sourcing in Rust and PostgreSQL.

Every financial movement (credit/debit) is stored as an immutable event. Account balances and statements are derived from read models updated asynchronously by a projection worker.

## Architecture

```
  POST /deposit ──▶ Command Handler ──▶ Event Store (PG) + Outbox (PG)
  POST /withdraw                              │
  POST /accounts                              │ pending events
                                              ▼
                                    Projection Worker
                                    poll outbox ──▶ update read models
                                              │
                                              ▼
  GET /balance ───▶ Query Handler ──▶ account_balances
  GET /statement                     transaction_history
```

- **Command Side**: validates business rules, appends immutable events to the event store
- **Projection Worker**: polls the outbox, processes events, updates read models
- **Query Side**: reads pre-computed data from optimized tables

## Tech Stack

| Crate | Role |
|---|---|
| axum | HTTP framework |
| tokio | Async runtime |
| sqlx | Compile-time checked SQL queries |
| serde | JSON serialization |
| thiserror / anyhow | Error handling |
| tracing | Structured logging |
| chrono | Date/time |
| uuid | Unique identifiers |

## Getting Started

```bash
cargo build
cargo run
```

## Project Status

Under active development. See [project.md](project.md) for the full RFC and implementation roadmap.
