# Database Schema

## Tables

### ledger_events

```sql
CREATE TABLE ledger_events (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_ledger_events_account
    ON ledger_events (account_id, created_at);
```

### event_outbox

```sql
CREATE TABLE event_outbox (
    id UUID PRIMARY KEY,
    event_id UUID NOT NULL REFERENCES ledger_events(id),
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at TIMESTAMPTZ
);

CREATE INDEX idx_event_outbox_pending
    ON event_outbox (status, created_at)
    WHERE status = 'pending';
```

### account_balances

```sql
CREATE TABLE account_balances (
    account_id UUID PRIMARY KEY,
    owner VARCHAR(255) NOT NULL,
    balance BIGINT NOT NULL DEFAULT 0,
    last_event_id UUID NOT NULL REFERENCES ledger_events(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### transaction_history

```sql
CREATE TABLE transaction_history (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    amount BIGINT NOT NULL,
    balance_after BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_transaction_history_account
    ON transaction_history (account_id, created_at DESC);
```

## How data flows through the tables

### 1. POST /accounts

```sql
INSERT INTO ledger_events (id, account_id, event_type, payload, created_at)
VALUES ('evt-001', 'acc-123', 'AccountCreated', '{"owner": "Francisco"}', now());

INSERT INTO event_outbox (id, event_id, status)
VALUES ('out-001', 'evt-001', 'pending');
```

After the worker processes:

```sql
INSERT INTO account_balances (account_id, owner, balance, last_event_id, updated_at)
VALUES ('acc-123', 'Francisco', 0, 'evt-001', now());
```

### 2. POST /accounts/:id/deposit (R$500.00)

```sql
INSERT INTO ledger_events (id, account_id, event_type, payload, created_at)
VALUES ('evt-002', 'acc-123', 'MoneyDeposited', '{"amount": 50000}', now());

INSERT INTO event_outbox (id, event_id, status)
VALUES ('out-002', 'evt-002', 'pending');
```

After the worker processes:

```sql
UPDATE account_balances
SET balance = balance + 50000,
    last_event_id = 'evt-002',
    updated_at = now()
WHERE account_id = 'acc-123';

INSERT INTO transaction_history (id, account_id, event_type, amount, balance_after, created_at)
VALUES ('tx-001', 'acc-123', 'MoneyDeposited', 50000, 50000, now());
```

### 3. POST /accounts/:id/withdraw (R$150.00)

```sql
INSERT INTO ledger_events (id, account_id, event_type, payload, created_at)
VALUES ('evt-003', 'acc-123', 'MoneyWithdrawn', '{"amount": 15000}', now());

INSERT INTO event_outbox (id, event_id, status)
VALUES ('out-003', 'evt-003', 'pending');
```

After the worker processes:

```sql
UPDATE account_balances
SET balance = balance - 15000,
    last_event_id = 'evt-003',
    updated_at = now()
WHERE account_id = 'acc-123';

INSERT INTO transaction_history (id, account_id, event_type, amount, balance_after, created_at)
VALUES ('tx-002', 'acc-123', 'MoneyWithdrawn', 15000, 35000, now());
```

### 4. GET /accounts/:id/balance

```sql
SELECT account_id, owner, balance, updated_at
FROM account_balances
WHERE account_id = 'acc-123';
```

```json
{
  "account_id": "acc-123",
  "owner": "Francisco",
  "balance": 35000,
  "updated_at": "2026-03-21T20:30:00Z"
}
```

### 5. GET /accounts/:id/statement

```sql
SELECT event_type, amount, balance_after, created_at
FROM transaction_history
WHERE account_id = 'acc-123'
ORDER BY created_at DESC;
```

```json
[
  { "event_type": "MoneyWithdrawn",  "amount": 15000, "balance_after": 35000, "created_at": "..." },
  { "event_type": "MoneyDeposited",  "amount": 50000, "balance_after": 50000, "created_at": "..." }
]
```

### 6. POST /accounts/:id/snapshot (replay)

Rebuilds balance from scratch by reading all events:

```sql
SELECT event_type, payload
FROM ledger_events
WHERE account_id = 'acc-123'
ORDER BY created_at;
```

```
AccountCreated  → balance = 0
MoneyDeposited  → balance = 0 + 50000 = 50000
MoneyWithdrawn  → balance = 50000 - 15000 = 35000
```

Then overwrites the read model:

```sql
UPDATE account_balances
SET balance = 35000,
    last_event_id = 'evt-003',
    updated_at = now()
WHERE account_id = 'acc-123';
```

## State of each table after this sequence

### ledger_events (3 rows, never modified)

| id | account_id | event_type | payload | created_at |
|---|---|---|---|---|
| evt-001 | acc-123 | AccountCreated | {"owner": "Francisco"} | 20:00:00 |
| evt-002 | acc-123 | MoneyDeposited | {"amount": 50000} | 20:01:00 |
| evt-003 | acc-123 | MoneyWithdrawn | {"amount": 15000} | 20:02:00 |

### event_outbox (3 rows, all done)

| id | event_id | status | created_at | processed_at |
|---|---|---|---|---|
| out-001 | evt-001 | done | 20:00:00 | 20:00:01 |
| out-002 | evt-002 | done | 20:01:00 | 20:01:01 |
| out-003 | evt-003 | done | 20:02:00 | 20:02:01 |

### account_balances (1 row, updated by worker)

| account_id | owner | balance | last_event_id | updated_at |
|---|---|---|---|---|
| acc-123 | Francisco | 35000 | evt-003 | 20:02:01 |

### transaction_history (2 rows)

| id | account_id | event_type | amount | balance_after | created_at |
|---|---|---|---|---|---|
| tx-001 | acc-123 | MoneyDeposited | 50000 | 50000 | 20:01:00 |
| tx-002 | acc-123 | MoneyWithdrawn | 15000 | 35000 | 20:02:00 |

## Index Strategy

| Index | Table | Purpose |
|---|---|---|
| `idx_ledger_events_account` | ledger_events | Fast replay by account |
| `idx_event_outbox_pending` | event_outbox | Worker finds pending events fast (partial index) |
| `idx_transaction_history_account` | transaction_history | Statement sorted by date |

The partial index on `event_outbox` (`WHERE status = 'pending'`) is key — as events get processed, the index shrinks. The worker only scans a small subset of rows.
