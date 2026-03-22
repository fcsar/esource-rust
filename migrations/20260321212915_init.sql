CREATE TABLE ledger_events (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_ledger_events_account
    ON ledger_events (account_id, created_at);

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

CREATE TABLE account_balances (
    account_id UUID PRIMARY KEY,
    owner VARCHAR(255) NOT NULL,
    balance BIGINT NOT NULL DEFAULT 0,
    last_event_id UUID NOT NULL REFERENCES ledger_events(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

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
