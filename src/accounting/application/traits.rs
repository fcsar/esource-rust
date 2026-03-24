use crate::accounting::domain::errors::LedgerError;
use crate::accounting::domain::event::Event;
use crate::accounting::domain::types::AccountId;

pub trait EventStore: Send + Sync {
    fn append(
        &self,
        event: &Event,
    ) -> impl std::future::Future<Output = Result<(), LedgerError>> + Send;

    fn load_events(
        &self,
        account_id: &AccountId,
    ) -> impl std::future::Future<Output = Result<Vec<Event>, LedgerError>> + Send;
}
