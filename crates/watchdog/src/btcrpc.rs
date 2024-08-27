use super::*;
use bitcoin::{Block, BlockHash};
use bitcoincore_rpc::{json::GetRawTransactionResult, Auth, Client, RpcApi};

pub struct BtcCli {
    rpc: Client,
}

impl BtcCli {
    pub fn new(url: &str, user: &str, pass: &str) -> Self {
        let rpc: Client =
            Client::new(url, Auth::UserPass(user.to_string(), pass.to_string())).unwrap();
        Self { rpc }
    }

    pub fn get_best_block_height(&self) -> Result<u64> {
        match self.rpc.get_block_count() {
            Ok(height) => Ok(height),
            Err(e) => Err(anyhow!("Failed to fetch block count: {:?}", e)),
        }
    }

    pub fn get_block(&self, height: u64) -> Result<Block> {
        let block_hash = self.rpc.get_block_hash(height).unwrap();
        match self.rpc.get_block(&block_hash) {
            Ok(block) => Ok(block),
            Err(e) => Err(anyhow!("Failed to fetch block: {:?}", e)),
        }
    }

    pub fn get_block_by_hash(&self, block_hash: BlockHash) -> Result<Block> {
        match self.rpc.get_block(&block_hash) {
            Ok(block) => Ok(block),
            Err(e) => Err(anyhow!("Failed to fetch block: {:?}", e)),
        }
    }

    pub fn get_unsepnt_tx_out(&self, txid: &bitcoin::Txid, vout: u32) {
        match self.rpc.get_tx_out(txid, vout, Some(true)) {
            Ok(Some(txout)) => {
                println!("TxOut found: {:?}", txout);
            }
            Ok(None) => {
                println!("No TxOut found for the given outpoint.");
            }
            Err(e) => {
                eprintln!("Error fetching TxOut: {:?}", e);
            }
        }
    }

    pub fn get_raw_transaction_info(
        &self,
        txid: &bitcoin::Txid,
    ) -> Result<(GetRawTransactionResult, Transaction)> {
        match self.rpc.get_raw_transaction_info(txid, None) {
            Ok(raw_tx) => match raw_tx.transaction() {
                Ok(tx) => Ok((raw_tx.clone(), tx.clone())),
                Err(e) => Err(anyhow!("Error convert raw tx to tx: {}", e)),
            },
            Err(e) => Err(anyhow!("Error fetching raw transaction: {}", e)),
        }
    }

    pub fn get_tx_out(&self, txid: &bitcoin::Txid, vout: u32) -> Result<TxOut> {
        match self.rpc.get_raw_transaction(txid, None) {
            Ok(raw_tx) => {
                let tx: bitcoin::Transaction = raw_tx;
                let tx_out = &tx.output[vout as usize];

                debug!("TxOut value: {}", tx_out.value);
                debug!("TxOut script_pubkey: {}", tx_out.script_pubkey);
                Ok(tx_out.clone())
            }
            Err(e) => Err(anyhow!("Error fetching raw transaction: {}", e)),
        }
    }

    pub fn get_tx_out_spent(&self, txid: &bitcoin::Txid, vout: u32) -> Result<bool> {
        match self.rpc.get_tx_out(txid, vout, None) {
            Ok(Some(_)) => Ok(false),
            Ok(None) => Ok(true),
            Err(e) => Err(anyhow!("Error fetching raw transaction: {}", e)),
        }
    }

    pub fn send_tx(&self, tx: &bitcoin::Transaction) -> Result<Txid> {
        match self.rpc.send_raw_transaction(tx) {
            Ok(txid) => Ok(txid),
            Err(e) => Err(anyhow!("send tx to node failed: {}", e)),
        }
    }

    // pub fn get_tx(&self, tx: &bitcoin::Transaction) -> Result<Txid> {
    //     match self.rpc.get_transaction(tx) {
    //         Ok(txid) => Ok(txid),
    //         Err(e) => Err(anyhow!("send tx to node failed: {}", e)),
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::BtcCli;
    use crate::config;
    use bitcoin::OutPoint;
    use std::str::FromStr;

    #[test]
    fn test_get_tx_out() {
        let cfg = config::load_config("./config.toml");
        let btc_cli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
        let outpoint = "903e78a5ce44c985459ff91fb9db49338b5901b8cfdbfa1aa875efc53eed4a2f:0";
        let out_point = OutPoint::from_str(outpoint).unwrap();

        let res = btc_cli.get_tx_out(&out_point.txid, out_point.vout);
        assert!(res.is_ok());
    }
}
