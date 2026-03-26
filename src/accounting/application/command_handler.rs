use crate::accounting::domain::aggregate::Account;
use crate::accounting::domain::command::Command;
use crate::accounting::domain::errors::LedgerError;
use crate::accounting::domain::event::Event;
use crate::accounting::domain::types::AccountId;

use super::traits::EventStore;

pub struct CommandHandler<S: EventStore> {
    store: S,
}

impl<S: EventStore> CommandHandler<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub async fn handle(&self, cmd: Command) -> Result<Event, LedgerError> {
        match cmd {
            Command::CreateAccount { owner } => {
                let account_id = AccountId::new();
                let account = Account::empty(account_id.clone());
                let event = account.create_account(owner)?;
                self.store.append(&event).await?;
                Ok(event)
            }
            Command::DepositMoney { account_id, amount } => {
                let events = self.store.load_events(&account_id).await?;
                let account = Account::rebuild(account_id, &events);
                let event = account.deposit(amount)?;
                self.store.append(&event).await?;
                Ok(event)
            }
            Command::WithdrawMoney { account_id, amount } => {
                let events = self.store.load_events(&account_id).await?;
                let account = Account::rebuild(account_id, &events);
                let event = account.withdraw(amount)?;
                self.store.append(&event).await?;
                Ok(event)
            }
        }
    }
}
