use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{AccountId, Amount, EventId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    AccountCreated {
        event_id: EventId,
        account_id: AccountId,
        owner: String,
        created_at: DateTime<Utc>,
    },
    MoneyDeposited {
        event_id: EventId,
        account_id: AccountId,
        amount: Amount,
        created_at: DateTime<Utc>,
    },
    MoneyWithdrawn {
        event_id: EventId,
        account_id: AccountId,
        amount: Amount,
        created_at: DateTime<Utc>,
    },
}
