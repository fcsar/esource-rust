# Project Structure

## Current (Phase 1 — Accounting only)

```
src/
│
├── main.rs
│
├── shared/
│   ├── mod.rs
│   └── types.rs                    → EventEnvelope, Metadata
│
└── accounting/
    ├── mod.rs
    │
    ├── domain/
    │   ├── mod.rs
    │   ├── types.rs                → AccountId, Amount, EventId
    │   ├── events.rs               → Event enum
    │   ├── commands.rs             → Command enum
    │   ├── errors.rs               → LedgerError enum
    │   └── aggregate.rs            → Account (rebuild, apply, validate)
    │
    ├── application/
    │   ├── mod.rs
    │   ├── traits.rs               → EventStore trait, ReadModelRepository trait
    │   ├── command_handler.rs      → receives Command, validates, emits Event
    │   └── query_handler.rs        → receives Query, returns read model data
    │
    ├── infrastructure/
    │   ├── mod.rs
    │   ├── postgres/
    │   │   ├── mod.rs
    │   │   ├── event_store.rs      → impl EventStore for PgEventStore
    │   │   ├── read_model.rs       → impl ReadModelRepository for PgReadModel
    │   │   └── migrations/
    │   │       └── 001_init.sql    → CREATE TABLE ledger_events, event_outbox, etc.
    │   └── http/
    │       ├── mod.rs
    │       ├── routes.rs           → Router (axum)
    │       └── handlers.rs         → POST /deposit, GET /balance, etc.
    │
    └── worker/
        ├── mod.rs
        └── projection.rs           → polling loop, event processing, read model update
```

## Scaled (Phase 4+ — multiple modules)

```
src/
│
├── main.rs
│
├── shared/
│   ├── mod.rs
│   ├── types.rs                    → EventEnvelope, Metadata
│   └── event_bus.rs                → EventListener trait, InMemoryEventBus
│
├── accounting/
│   ├── mod.rs
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── types.rs                → AccountId, Amount, EventId
│   │   ├── events.rs               → Event enum
│   │   ├── commands.rs             → Command enum
│   │   ├── errors.rs               → LedgerError enum
│   │   └── aggregate.rs            → Account
│   ├── application/
│   │   ├── mod.rs
│   │   ├── traits.rs               → EventStore, ReadModelRepository
│   │   ├── command_handler.rs
│   │   └── query_handler.rs
│   ├── infrastructure/
│   │   ├── mod.rs
│   │   ├── postgres/
│   │   │   ├── mod.rs
│   │   │   ├── event_store.rs
│   │   │   ├── read_model.rs
│   │   │   └── migrations/
│   │   └── http/
│   │       ├── mod.rs
│   │       ├── routes.rs
│   │       └── handlers.rs
│   └── worker/
│       ├── mod.rs
│       └── projection.rs
│
├── notification/
│   ├── mod.rs
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── types.rs                → NotificationId, Channel (Push, Email, SMS)
│   │   └── events.rs               → NotificationSent, NotificationFailed
│   ├── application/
│   │   ├── mod.rs
│   │   └── notification_handler.rs → listens to MoneyDeposited, sends push
│   └── infrastructure/
│       ├── mod.rs
│       └── push_service.rs         → Firebase, APNs, etc.
│
└── reporting/
    ├── mod.rs
    ├── domain/
    │   ├── mod.rs
    │   └── types.rs                → TaxReport, MonthlyStatement
    ├── application/
    │   ├── mod.rs
    │   └── report_generator.rs     → listens to all events, aggregates data
    └── infrastructure/
        ├── mod.rs
        └── postgres/
            └── report_store.rs     → stores generated reports
```

## Module dependency rules

```
                    ┌────────────────┐
                    │     shared     │
                    │                │
                    │ EventEnvelope  │
                    │ EventListener  │
                    │ EventBus       │
                    └───────┬────────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
              ▼             ▼             ▼
     ┌────────────┐ ┌──────────────┐ ┌───────────┐
     │ accounting │ │ notification │ │ reporting │
     │            │ │              │ │           │
     │ imports:   │ │ imports:     │ │ imports:  │
     │  shared    │ │  shared      │ │  shared   │
     │            │ │              │ │           │
     │ never:     │ │ never:       │ │ never:    │
     │ notificat. │ │  accounting  │ │ account.  │
     │ reporting  │ │  reporting   │ │ notific.  │
     └────────────┘ └──────────────┘ └───────────┘
```

## Layer dependency rules (inside each module)

```
  infrastructure ──▶ application ──▶ domain
        │                │              │
   knows axum       knows traits    knows nothing
   knows sqlx       knows domain    pure Rust
   knows domain     never infra     no dependencies
```

## Composition in main.rs

```
main.rs
  │
  ├── create PgPool
  │
  ├── init accounting module
  │     ├── PgEventStore::new(pool)
  │     ├── PgReadModel::new(pool)
  │     ├── CommandHandler::new(event_store)
  │     ├── QueryHandler::new(read_model)
  │     ├── spawn projection worker
  │     └── register HTTP routes
  │
  ├── init notification module (Phase 4+)
  │     ├── PushService::new(config)
  │     ├── NotificationHandler::new(push_service)
  │     └── register as EventListener
  │
  ├── init event bus (Phase 4+)
  │     ├── InMemoryEventBus::new()          ← monolith
  │     └── KafkaEventBus::new(broker_url)   ← after extraction
  │
  └── start axum server
```

## Migration path: monolith → microservices

```
Step 1: Modular monolith (current)
  ┌──────────────────────────────────┐
  │ single binary                    │
  │ accounting + notification + ...  │
  │ InMemoryEventBus                 │
  └──────────────────────────────────┘

Step 2: Introduce Kafka (Phase 4)
  ┌──────────────────────────────────┐
  │ single binary                    │
  │ accounting + notification + ...  │
  │ KafkaEventBus (replaces in-mem)  │
  └───────────────┬──────────────────┘
                  │
                  ▼
            ┌──────────┐
            │  Kafka   │
            └──────────┘

Step 3: Extract module as microservice
  ┌──────────────────────┐
  │ edriven              │
  │ accounting + report. │
  └──────────┬───────────┘
             │ publish
             ▼
       ┌──────────┐
       │  Kafka   │
       └─────┬────┘
             │ consume
             ▼
  ┌──────────────────────┐
  │ notification-svc     │
  │ (separate binary)    │
  └──────────────────────┘
```
