use crate::repository::{CreatedJournal, Entries, Entry, NewJournal};

#[derive(Clone)]
pub struct EntryService<R: Entries>(R);

#[derive(Debug)]
pub enum CreateError {
    Invalid,
    Database(sqlx::Error),
}

impl<R: Entries> EntryService<R> {
    pub fn new(repository: R) -> Self {
        Self(repository)
    }

    pub async fn get_entry(&self, id: i64) -> Result<Option<Entry>, sqlx::Error> {
        self.0.find_by_id(id).await
    }

    pub async fn create_journal(&self, journal: NewJournal) -> Result<CreatedJournal, CreateError> {
        if !(2..=100).contains(&journal.entries.len()) {
            return Err(CreateError::Invalid);
        }
        let mut balance = 0_i128;
        for entry in &journal.entries {
            if entry.account_code.is_empty()
                || entry.account_code.chars().count() > 20
                || entry.description.chars().count() > 255
                || entry.amount_cents == 0
            {
                return Err(CreateError::Invalid);
            }
            balance += i128::from(entry.amount_cents);
        }
        if balance != 0 {
            return Err(CreateError::Invalid);
        }
        self.0
            .create_journal(journal)
            .await
            .map_err(CreateError::Database)
    }
}
