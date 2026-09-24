use std::future::Future;

use serde::{Deserialize, Serialize};
use sqlx::{MySql, Pool};

#[derive(Debug, PartialEq, sqlx::FromRow, Serialize, utoipa::ToSchema)]
pub struct Entry {
    pub id: i64,
    pub journal_id: i64,
    pub account_code: String,
    pub amount_cents: i64,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, utoipa::ToSchema)]
pub struct NewEntry {
    pub account_code: String,
    pub amount_cents: i64,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, utoipa::ToSchema)]
pub struct NewJournal {
    pub entries: Vec<NewEntry>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreatedJournal {
    pub id: i64,
    pub entry_ids: Vec<i64>,
}

pub trait Entries: Clone + Send + Sync + 'static {
    fn find_by_id(
        &self,
        id: i64,
    ) -> impl Future<Output = Result<Option<Entry>, sqlx::Error>> + Send;

    fn create_journal(
        &self,
        journal: NewJournal,
    ) -> impl Future<Output = Result<CreatedJournal, sqlx::Error>> + Send;
}

#[derive(Clone)]
pub struct EntryRepository(Pool<MySql>);

impl EntryRepository {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self(pool)
    }
}

impl Entries for EntryRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Entry>, sqlx::Error> {
        sqlx::query_as::<_, Entry>(
            "SELECT id, journal_id, account_code, amount_cents, description FROM journal_entries WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.0)
        .await
    }

    async fn create_journal(&self, journal: NewJournal) -> Result<CreatedJournal, sqlx::Error> {
        let mut transaction = self.0.begin().await?;
        let result = sqlx::query("INSERT INTO journals () VALUES ()")
            .execute(&mut *transaction)
            .await?;
        let id = result.last_insert_id() as i64;
        let mut entry_ids = Vec::with_capacity(journal.entries.len());
        for entry in journal.entries {
            let result = sqlx::query("INSERT INTO journal_entries (journal_id, account_code, amount_cents, description) VALUES (?, ?, ?, ?)")
                .bind(id)
                .bind(entry.account_code)
                .bind(entry.amount_cents)
                .bind(entry.description)
                .execute(&mut *transaction)
                .await?;
            entry_ids.push(result.last_insert_id() as i64);
        }
        transaction.commit().await?;
        Ok(CreatedJournal { id, entry_ids })
    }
}
