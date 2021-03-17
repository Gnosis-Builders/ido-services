mod signatures;

use anyhow::Result;
use sqlx::PgPool;

pub use signatures::*;

// The pool uses an Arc internally.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

// The implementation is split up into several modules which contain more public methods.
impl Database {
    pub fn new(uri: &str) -> Result<Self> {
        Ok(Self {
            pool: PgPool::connect_lazy(uri)?,
        })
    }

    /// Delete all data in the database. Only used by tests.
    pub async fn clear(&self) -> Result<()> {
        use sqlx::Executor;
        self.pool
            .execute(sqlx::query("TRUNCATE signatures;"))
            .await?;
        Ok(())
    }
}
