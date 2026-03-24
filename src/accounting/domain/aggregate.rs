use chrono::Utc;

use super::errors::LedgerError;
use super::event::Event;
use super::types::{AccountId, Amount, EventId};

#[derive(Debug, Clone)]
pub struct Account {
    pub id: AccountId,
    pub owner: String,
    pub balance: Amount,
    pub exists: bool,
}

impl Account {
    pub fn empty(id: AccountId) -> Self {
        Self {
            id,
            owner: String::new(),
            balance: Amount(0),
            exists: false,
        }
    }

    pub fn rebuild(id: AccountId, events: &[Event]) -> Self {
        let mut account = Self::empty(id);
        for event in events {
            account.apply(event);
        }
        account
    }

    fn apply(&mut self, event: &Event) {
        match event {
            Event::AccountCreated { owner, .. } => {
                self.owner = owner.clone();
                self.exists = true;
            }
            Event::MoneyDeposited { amount, .. } => {
                self.balance = Amount(self.balance.0 + amount.0);
            }
            Event::MoneyWithdrawn { amount, .. } => {
                self.balance = Amount(self.balance.0 - amount.0);
            }
        }
    }

    pub fn create_account(&self, owner: String) -> Result<Event, LedgerError> {
        if self.exists {
            return Err(LedgerError::DuplicateAccount(self.id.clone()));
        }

        Ok(Event::AccountCreated {
            event_id: EventId::new(),
            account_id: self.id.clone(),
            owner,
            created_at: Utc::now(),
        })
    }

    pub fn deposit(&self, amount: Amount) -> Result<Event, LedgerError> {
        if !self.exists {
            return Err(LedgerError::AccountNotFound(self.id.clone()));
        }

        if amount.0 <= 0 {
            return Err(LedgerError::InvalidAmount);
        }

        Ok(Event::MoneyDeposited {
            event_id: EventId::new(),
            account_id: self.id.clone(),
            amount,
            created_at: Utc::now(),
        })
    }

    pub fn withdraw(&self, amount: Amount) -> Result<Event, LedgerError> {
        if !self.exists {
            return Err(LedgerError::AccountNotFound(self.id.clone()));
        }

        if amount.0 <= 0 {
            return Err(LedgerError::InvalidAmount);
        }

        if self.balance.0 < amount.0 {
            return Err(LedgerError::InsufficientFunds {
                available: self.balance,
                requested: amount,
            });
        }

        Ok(Event::MoneyWithdrawn {
            event_id: EventId::new(),
            account_id: self.id.clone(),
            amount,
            created_at: Utc::now(),
        })
    }
}
