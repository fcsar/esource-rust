use serde::Deserialize;

use super::types::{AccountId, Amount};

#[derive(Debug, Deserialize)]
pub enum Command {
    CreateAccount {
        owner: String,
    },
    DepositMoney {
        account_id: AccountId,
        amount: Amount,
    },
    WithdrawMoney {
        account_id: AccountId,
        amount: Amount,
    },
}
