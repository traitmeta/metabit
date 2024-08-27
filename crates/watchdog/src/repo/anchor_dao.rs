use anchor::AnchorTxOut;

use super::*;

impl Dao {
    pub async fn insert_anchor_tx_out(&self, info: AnchorTxOut) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO anchor_tx_out (tx_id, vout, value,script_pubkey,unlock_info,spent,confirmed_block_height) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&info.tx_id)
            .bind(info.vout)
            .bind(info.value)
            .bind(&info.script_pubkey)
            .bind(&info.unlock_info)
            .bind(info.spent)
            .bind(info.confirmed_block_height)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_old_sanchor_tx_out(
        &self,
        current_block_height: i64,
    ) -> Result<Vec<AnchorTxOut>, sqlx::Error> {
        let expired_block_height = current_block_height - 16;
        let resp_data: Vec<AnchorTxOut> =
            sqlx::query_as("SELECT * FROM anchor_tx_out WHERE spent = ? and confirmed_block_height > ? and confirmed_block_height < ?")
                .bind(false)
                .bind(0)
                .bind(expired_block_height)
                .fetch_all(&self.pool)
                .await?;

        Ok(resp_data)
    }

    pub async fn get_anchor_tx_out(
        &self,
        current_block_height: i64,
    ) -> Result<Vec<AnchorTxOut>, sqlx::Error> {
        let expired_block_height = current_block_height - 16;
        let resp_data: Vec<AnchorTxOut> = sqlx::query_as(
            "SELECT * FROM anchor_tx_out WHERE spent = ? and confirmed_block_height = ?",
        )
        .bind(false)
        .bind(expired_block_height)
        .fetch_all(&self.pool)
        .await?;

        Ok(resp_data)
    }

    pub async fn get_unpent_comfiremd_anchor_tx_out(
        &self,
    ) -> Result<Vec<AnchorTxOut>, sqlx::Error> {
        let resp_data: Vec<AnchorTxOut> = sqlx::query_as(
            "SELECT * FROM anchor_tx_out WHERE spent = $1 and confirmed_block_height = $2 order by create_at desc limit 100",
        )
        .bind(false)
        .bind(0)
        .fetch_all(&self.pool)
        .await?;

        Ok(resp_data)
    }

    pub async fn update_anchor_tx_out(&self, block_height: i64, txids: Vec<String>) -> Result<u64> {
        let rows_affected = sqlx::query!(
            "UPDATE anchor_tx_out SET confirmed_block_height = $1 WHERE tx_id in ($2)",
            block_height,
            txids.join(",")
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected)
    }

    pub async fn update_anchor_tx_confirmed_height(
        &self,
        block_height: i64,
        txid: String,
    ) -> Result<u64> {
        let rows_affected = sqlx::query!(
            "UPDATE anchor_tx_out SET confirmed_block_height = $1 WHERE tx_id = $2",
            block_height,
            txid
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected)
    }

    pub async fn update_anchor_tx_out_spent(&self, txid: String, vout: i32) -> Result<u64> {
        let rows_affected = sqlx::query!(
            "UPDATE anchor_tx_out SET spent = $1 WHERE tx_id = $2 and vout = $3",
            true,
            txid,
            vout
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected)
    }
}
