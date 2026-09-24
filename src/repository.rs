use std::future::Future;

use sqlx::{MySql, Pool};

#[derive(Debug, PartialEq, sqlx::FromRow, serde::Serialize, utoipa::ToSchema)]
pub struct Entry {
    pub id: i64,
    pub account_code: String,
    pub amount_cents: i64,
    pub description: String,
}

pub trait Entries: Clone + Send + Sync + 'static {
    fn find_by_id(
        &self,
        id: i64,
    ) -> impl Future<Output = Result<Option<Entry>, sqlx::Error>> + Send;
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
            "SELECT id, account_code, amount_cents, description FROM journal_entries WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.0)
        .await
    }
}
