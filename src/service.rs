use crate::domain::{CreatedJournal, Entries, Entry, NewJournal};

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

    /// 検証に失敗した仕訳はリポジトリに渡さず、`CreateError::Invalid` を返す。
    pub async fn create_journal(&self, journal: NewJournal) -> Result<CreatedJournal, CreateError> {
        if !journal.is_valid() {
            return Err(CreateError::Invalid);
        }
        self.0
            .create_journal(journal)
            .await
            .map_err(CreateError::Database)
    }
}
