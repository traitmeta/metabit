pub mod anchor;
pub mod anchor_dao;
pub mod indexer;
pub mod indexer_dao;

use super::*;
use crate::config;
use sqlx::{postgres::PgPool, Executor, FromRow, Pool, Postgres};

pub async fn conn_pool(cfg: &config::DBConfig) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPool::connect(&cfg.url).await
}

pub async fn create_table(pool: Pool<Postgres>) -> Result<(), sqlx::Error> {
    pool.execute(
        "CREATE TABLE IF NOT EXISTS anchor_tx_out (
            tx_id TEXT,
            vout INTEGER,
            value BIGINT,
            script_pubkey TEXT,
            unlock_info TEXT,
            spent BOOLEAN,
            confirmed_block_height BIGINT
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS indexer (
        height BIGINT,
        hash TEXT,
        chain_name TEXT
    )",
    )
    .await?;

    Ok(())
}

pub struct Dao {
    pool: Pool<Postgres>,
}

impl Dao {
    pub fn new(pool: Pool<Postgres>) -> Dao {
        Dao { pool }
    }
}
