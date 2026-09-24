use crate::repository::{Entries, Entry};

#[derive(Clone)]
pub struct EntryService<R: Entries>(R);

impl<R: Entries> EntryService<R> {
    pub fn new(repository: R) -> Self {
        Self(repository)
    }

    pub async fn get_entry(&self, id: i64) -> Result<Option<Entry>, sqlx::Error> {
        self.0.find_by_id(id).await
    }
}
