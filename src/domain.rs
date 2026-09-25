use std::future::Future;

use serde::{Deserialize, Serialize};

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

impl NewJournal {
    /// 文字数は `chars()` で数え、MySQLの `VARCHAR(n)` と同じく文字単位で上限を判定する。
    /// 合計は `i128` で計算するため、`i64` の金額を100行足しても桁あふれしない。
    pub fn is_valid(&self) -> bool {
        if !(2..=100).contains(&self.entries.len()) {
            return false;
        }
        let mut balance = 0_i128;
        for entry in &self.entries {
            if entry.account_code.is_empty()
                || entry.account_code.chars().count() > 20
                || entry.description.chars().count() > 255
                || entry.amount_cents == 0
            {
                return false;
            }
            balance += i128::from(entry.amount_cents);
        }
        balance == 0
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreatedJournal {
    pub id: i64,
    pub entry_ids: Vec<i64>,
}

pub trait Entries: Clone + Send + Sync + 'static {
    /// 該当する行が無い場合は `Err` ではなく `Ok(None)` を返す。
    fn find_by_id(
        &self,
        id: i64,
    ) -> impl Future<Output = Result<Option<Entry>, sqlx::Error>> + Send;

    /// 検証は呼び出し側の責務で、ここでは行わない。全行を1トランザクションで保存し、
    /// `entry_ids` は入力と同じ順に並ぶ。
    fn create_journal(
        &self,
        journal: NewJournal,
    ) -> impl Future<Output = Result<CreatedJournal, sqlx::Error>> + Send;
}
