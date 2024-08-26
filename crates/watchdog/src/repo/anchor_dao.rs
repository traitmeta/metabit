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
}
