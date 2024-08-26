use repo::indexer::Indexer;

use super::*;

pub struct Syncer {
    btccli: btcrpc::BtcCli,
    dao: Arc<Dao>,
}

impl Syncer {
    pub fn new(btccli: btcrpc::BtcCli, dao: Arc<Dao>) -> Self {
        Syncer { btccli, dao }
    }

    pub async fn sync_block(&self) -> Result<()> {
        let tip_height = self.btccli.get_best_block_height().unwrap();
        let last_height = self
            .dao
            .get_indexer("bitcoin_main".to_string())
            .await
            .unwrap_or(Indexer::default());
        for height in (last_height.height as u64 + 1)..=tip_height {
            match self.btccli.get_block(height as u64) {
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
}
