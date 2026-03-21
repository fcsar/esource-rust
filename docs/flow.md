# Event Flow

## Complete Deposit Flow

```
   Client                    Command Side                    PostgreSQL
     │                           │                               │
     │  POST /deposit            │                               │
     │  { "amount": 50000 }      │                               │
     │ ─────────────────────▶    │                               │
     │                           │                               │
     │                      Validate:                            │
     │                      - account exists?                    │
     │                      - amount > 0?                        │
     │                           │                               │
     │                           │   BEGIN                       │
     │                           │ ─────────────────────────▶    │
     │                           │                               │
     │                           │   INSERT ledger_events        │
     │                           │   (MoneyDeposited, 50000)     │
     │                           │ ─────────────────────────▶    │
     │                           │                               │
     │                           │   INSERT event_outbox         │
     │                           │   (status: pending)           │
     │                           │ ─────────────────────────▶    │
     │                           │                               │
     │                           │   COMMIT                      │
     │                           │ ─────────────────────────▶    │
     │                           │                               │
     │  201 Created              │                               │
     │  { event_id, event }      │                               │
     │ ◀─────────────────────    │                               │
     │                           │                               │
     │                                                           │
     │            ┌──── eventual consistency gap ────┐           │
     │            │     (~50-100ms)                  │           │
     │            └──────────────────────────────────┘           │
     │                                                           │
     │                     Projection Worker                     │
     │                           │                               │
     │                           │   SELECT event_outbox         │
     │                           │   WHERE status = 'pending'    │
     │                           │ ─────────────────────────▶    │
     │                           │                               │
     │                           │   ◀── found evt-001 ──────   │
     │                           │                               │
     │                           │   UPDATE outbox               │
     │                           │   SET status = 'processing'   │
     │                           │ ─────────────────────────▶    │
     │                           │                               │
     │                           │   UPDATE account_balances     │
     │                           │   SET balance = balance+50000 │
     │                           │ ─────────────────────────▶    │
     │                           │                               │
     │                           │   INSERT transaction_history  │
     │                           │ ─────────────────────────▶    │
     │                           │                               │
     │                           │   UPDATE outbox               │
     │                           │   SET status = 'done'         │
     │                           │ ─────────────────────────▶    │
     │                                                           │
     │                                                           │
     │  GET /balance             Query Side                      │
     │ ─────────────────────▶    │                               │
     │                           │   SELECT account_balances     │
     │                           │ ─────────────────────────▶    │
     │                           │   ◀── balance: 50000 ─────   │
     │  { balance: 50000 }       │                               │
     │ ◀─────────────────────    │                               │
```

## Table State at Each Step

### After POST /deposit (before worker)

```
ledger_events
┌─────────┬──────────┬────────────────┬──────────────────┐
│ id      │ account  │ event_type     │ payload          │
├─────────┼──────────┼────────────────┼──────────────────┤
│ evt-001 │ acc-123  │ AccountCreated │ {"owner":"Fran"} │
│ evt-002 │ acc-123  │ MoneyDeposited │ {"amount":50000} │
└─────────┴──────────┴────────────────┴──────────────────┘

event_outbox
┌─────────┬──────────┬─────────┐
│ id      │ event_id │ status  │
├─────────┼──────────┼─────────┤
│ out-001 │ evt-001  │ done    │
│ out-002 │ evt-002  │ pending │  ◀── waiting to be processed
└─────────┴──────────┴─────────┘

account_balances
┌──────────┬───────┬─────────┐
│ account  │ owner │ balance │
├──────────┼───────┼─────────┤
│ acc-123  │ Fran  │ 0       │  ◀── still zero!
└──────────┴───────┴─────────┘
```

### After worker processes

```
event_outbox
┌─────────┬──────────┬────────┐
│ id      │ event_id │ status │
├─────────┼──────────┼────────┤
│ out-001 │ evt-001  │ done   │
│ out-002 │ evt-002  │ done   │  ◀── processed
└─────────┴──────────┴────────┘

account_balances
┌──────────┬───────┬─────────┐
│ account  │ owner │ balance │
├──────────┼───────┼─────────┤
│ acc-123  │ Fran  │ 50000   │  ◀── updated!
└──────────┴───────┴─────────┘

transaction_history
┌──────────┬────────────────┬────────┬───────────────┐
│ account  │ event_type     │ amount │ balance_after │
├──────────┼────────────────┼────────┼───────────────┤
│ acc-123  │ MoneyDeposited │ 50000  │ 50000         │
└──────────┴────────────────┴────────┴───────────────┘
```

## Worker Lifecycle

```
                    ┌─────────────────────┐
                    │   Worker starts     │
                    └──────────┬──────────┘
                               │
                               ▼
                    ┌─────────────────────┐
              ┌───▶ │  Poll event_outbox   │
              │     │  status = 'pending'  │
              │     └──────────┬──────────┘
              │                │
              │         found events?
              │        ╱            ╲
              │      no              yes
              │      │                │
              │      ▼                ▼
              │  ┌────────┐   ┌──────────────┐
              │  │sleep   │   │ for each:    │
              │  │100ms   │   │              │
              │  └───┬────┘   │ 1. pending   │
              │      │        │    → processing│
              │      │        │ 2. load event │
              │      │        │ 3. match type │
              │      │        │ 4. update     │
              │      │        │    read model │
              │      │        │ 5. processing │
              │      │        │    → done     │
              │      │        └──────┬───────┘
              │      │               │
              └──────┴───────────────┘
```

## Failure Recovery

### Worker crashes before processing

```
event_outbox: status = 'pending'
              ──▶ worker restarts ──▶ picks it up again ✅
```

### Worker crashes during processing

```
event_outbox: status = 'processing'
              ──▶ worker restarts
              ──▶ sees 'processing' stuck for too long
              ──▶ reprocesses (idempotent) ✅
```

### Worker crashes after updating read model but before marking done

```
event_outbox: status = 'processing'
account_balances: already updated
              ──▶ worker restarts
              ──▶ reprocesses
              ──▶ idempotency check: last_event_id already matches
              ──▶ skips update, marks 'done' ✅
```

### Database crashes during transaction

```
BEGIN
  INSERT ledger_events   ✅
  INSERT event_outbox    ← crash here
ROLLBACK (automatic)

Neither row exists. Nothing to process.
Client receives error, can retry. ✅
```

## Full System Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    edriven process                        │
│                                                          │
│  ┌─────────────────────┐   ┌──────────────────────────┐  │
│  │     HTTP Server      │   │   Projection Worker      │  │
│  │     (axum)           │   │   (tokio::spawn)         │  │
│  │                      │   │                          │  │
│  │  POST /accounts      │   │   loop {                 │  │
│  │  POST /deposit       │   │     poll outbox          │  │
│  │  POST /withdraw      │   │     match event_type {   │  │
│  │  GET  /balance       │   │       AccountCreated     │  │
│  │  GET  /statement     │   │       MoneyDeposited     │  │
│  │  POST /snapshot      │   │       MoneyWithdrawn     │  │
│  │  GET  /health        │   │     }                    │  │
│  │                      │   │     update read models   │  │
│  └──────────┬───────────┘   └────────────┬─────────────┘  │
│             │                            │                │
│             └──────────┬─────────────────┘                │
│                        │                                  │
│                   ┌────┴────┐                             │
│                   │  PgPool │                             │
│                   └────┬────┘                             │
└────────────────────────┼─────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────┐
│                    PostgreSQL                             │
│                                                          │
│  ┌──────────────┐  ┌─────────────┐  ┌────────────────┐  │
│  │ledger_events │  │event_outbox │  │account_balances│  │
│  │(append-only) │  │(work queue) │  │(read model)    │  │
│  └──────────────┘  └─────────────┘  └────────────────┘  │
│                                                          │
│  ┌─────────────────────┐                                 │
│  │transaction_history  │                                 │
│  │(read model)         │                                 │
│  └─────────────────────┘                                 │
└──────────────────────────────────────────────────────────┘
```
