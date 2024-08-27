use super::*;
use btcrpc::BtcCli;
use repo::indexer::Indexer;
use std::str::FromStr;

pub struct Syncer {
    btccli: btcrpc::BtcCli,
    dao: Arc<Dao>,
}

impl Syncer {
    pub async fn new(cfg: &config::Config) -> Self {
        let btccli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
        let conn_pool = repo::conn_pool(&cfg.database).await.unwrap();
        let dao = Dao::new(conn_pool);
        Self {
            btccli,
            dao: Arc::new(dao),
        }
    }

    pub async fn sync_block(&self) -> Result<()> {
        let tip_height = self.btccli.get_best_block_height().unwrap();
        let last_height = self
            .dao
            .get_indexer("bitcoin_main".to_string())
            .await
            .unwrap_or(Indexer::default());
        for height in (last_height.height as u64 + 1)..=tip_height {
            match self.btccli.get_block(height) {
                Ok(block) => {
                    // TODO some logic here
                    // 1. update the spent or block height for anchor tx
                    let mut spent_outpoints = vec![];
                    let mut txs = vec![];
                    for tx in block.txdata.iter() {
                        for input in tx.input.iter() {
                            spent_outpoints.push(input.previous_output.to_string());
                        }

                        txs.push(tx.compute_txid());
                    }

                    // 2. update anchor out spent

                    // 3. update anchor tx confirmed

                    self.dao
                        .insert_indexer(
                            height as i64,
                            block.block_hash().to_string(),
                            "bitcoin_main".to_string(),
                        )
                        .await
                        .unwrap();
                }
                Err(e) => {
                    error!("get block {} failed : {}", height, e);
                }
            }
        }

        Ok(())
    }

    // TODO feerate < 1 set confirmed to -1
    pub async fn sync_anchor(&self) {
        let tx_outs = self.dao.get_unpent_comfiremd_anchor_tx_out().await.unwrap();
        for out in tx_outs.iter() {
            let txid = Txid::from_str(out.tx_id.as_str()).unwrap();
            match self.btccli.get_raw_transaction_info(&txid) {
                Ok((raw_tx, _)) => {
                    if raw_tx.blockhash.is_none() {
                        continue;
                    }

                    let blockhash = raw_tx.blockhash.unwrap();
                    if let Ok(block) = self.btccli.get_block_by_hash(blockhash) {
                        match self
                            .dao
                            .update_anchor_tx_confirmed_height(
                                block.bip34_block_height().unwrap() as i64,
                                raw_tx.txid.to_string(),
                            )
                            .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                error!("update_anchor_tx_confirmed_height failed : {}", e)
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("get tx out {} vout {} failed : {}", txid, out.vout, e);
                }
            }

            match self.btccli.get_tx_out_spent(&txid, out.vout as u32) {
                Ok(spent) => {
                    if spent {
                        match self
                            .dao
                            .update_anchor_tx_out_spent(out.tx_id.clone(), out.vout)
                            .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                error!("update_anchor_tx_out_spent failed : {}", e)
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("get tx out {} vout {} failed : {}", txid, out.vout, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sync_anchor() {
        let cfg = config::load_config("./config.toml");
        let syncer = Syncer::new(&cfg).await;
        let res = syncer.sync_anchor().await;
    }
}
