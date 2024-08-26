use indexer::Indexer;

use super::*;

impl Dao {
    pub async fn insert_indexer(
        &self,
        height: i64,
        hash: String,
        chain_name: String,
    ) -> Result<u64> {
        let rows_affected = sqlx::query!(
            "UPDATE indexer SET height = $1, hash = $2 WHERE chain_name = $3",
            height,
            hash,
            chain_name
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        println!(
            "Number of rows affected about insert indexer: {}",
            rows_affected
        );

        Ok(rows_affected)
    }

    pub async fn get_indexer(&self, chain_name: String) -> Result<Indexer> {
        let resp_data: Indexer = sqlx::query_as("SELECT * FROM indexer WHERE chain_name = $1")
            .bind(chain_name)
            .fetch_one(&self.pool)
            .await?;

        Ok(resp_data)
    }
}
