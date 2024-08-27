use std::{collections::HashMap, str::FromStr};

use bitcoin::{consensus::encode::serialize_hex, Amount, OutPoint, ScriptBuf};
use bittx::{build_helper, signer};
use btcrpc::BtcCli;
use datatypes::types;

use super::*;

pub struct TxSender {
    btccli: btcrpc::BtcCli,
    receiver: String,
    wif: String,
    dao: Arc<repo::Dao>,
}

impl TxSender {
    pub async fn new(cfg: &config::Config) -> Self {
        let btccli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
        let conn_pool = repo::conn_pool(&cfg.database).await.unwrap();
        let dao = Dao::new(conn_pool);
        Self {
            btccli,
            receiver: cfg.sign.receiver.clone(),
            wif: cfg.sign.wif.clone(),
            dao: Arc::new(dao),
        }
    }

    pub fn send(&self, tx: Transaction) -> Result<Txid> {
        self.btccli.send_tx(&tx)
    }

    pub async fn send_task(&self) -> Result<()> {
        let height = self.btccli.get_best_block_height();
        if height.is_err() {
            return Err(anyhow!("get block height failed"));
        }

        let height = height.unwrap();
        let txouts = self.dao.get_anchor_tx_out(height as i64).await;
        if txouts.is_err() {
            return Err(anyhow!("get anchor txouts failed"));
        }

        let txouts = txouts.unwrap();
        let mut datas = HashMap::new();

        for tx_out in txouts.into_iter() {
            datas
                .entry(tx_out.tx_id.clone())
                .or_insert_with(Vec::new)
                .push(tx_out);
        }

        for (tx_id, tx_outs) in &datas {
            let mut unlock_infos = vec![];
            let mut unlock_outs = vec![];
            for tx_out in tx_outs {
                let unlock_hex = hex::decode(tx_out.unlock_info.clone()).unwrap();
                unlock_infos.push(unlock_hex);
                let out_point = OutPoint::from_str(&format!("{}:{}", tx_id, tx_out.vout)).unwrap();
                let out = TxOut {
                    value: Amount::from_sat(tx_out.value as u64),
                    script_pubkey: ScriptBuf::from_hex(&tx_out.script_pubkey).unwrap(),
                };
                unlock_outs.push((out, out_point));
            }

            let anchor_info = types::AnchorInfo {
                anchor_txid: tx_id.to_string(),
                unlock_bytes: unlock_infos,
                unlock_outs,
                recipient: self.receiver.clone(),
            };
            let signed_tx = self.build_and_sign(anchor_info).await?;
            info!("send transaction started: {:?}", signed_tx.compute_txid());
            self.send(signed_tx.clone())?;
        }
        Ok(())
    }

    pub async fn send_task_by_hash(&self, tx_id: String) -> Result<Transaction> {
        let txouts = self.dao.get_anchor_tx_out_by_tx_id(tx_id.clone()).await;
        if txouts.is_err() {
            return Err(anyhow!("get anchor txouts failed"));
        }

        let txouts = txouts.unwrap();
        let mut unlock_infos = vec![];
        let mut unlock_outs = vec![];
        for tx_out in txouts.into_iter() {
            let unlock_hex = hex::decode(tx_out.unlock_info.clone()).unwrap();
            unlock_infos.push(unlock_hex);
            let out_point = OutPoint::from_str(&format!("{}:{}", tx_id, tx_out.vout)).unwrap();
            let out = TxOut {
                value: Amount::from_sat(tx_out.value as u64),
                script_pubkey: ScriptBuf::from_hex(&tx_out.script_pubkey).unwrap(),
            };
            unlock_outs.push((out, out_point));
        }

        let anchor_info = types::AnchorInfo {
            anchor_txid: tx_id.to_string(),
            unlock_bytes: unlock_infos,
            unlock_outs,
            recipient: self.receiver.clone(),
        };

        self.build_and_sign(anchor_info).await
    }

    pub async fn build_sign_and_send(&self, anchor_info: types::AnchorInfo) -> Result<Txid> {
        let (anchor_tx, prevouts) = build_helper::build_anchor_tx(anchor_info).await?;
        let signed_tx = signer::sign_tx(self.wif.clone(), anchor_tx, prevouts, vec![0]).await?;
        println!("{}", serialize_hex(&signed_tx));
        loop {
            match self.send(signed_tx.clone()) {
                Ok(txid) => {
                    println!("{}", txid);
                    return Ok(txid);
                }
                Err(e) => {
                    println!("send tx error: {}", e);
                    if e.to_string().contains("insufficient fee")
                        || e.to_string().contains("bad-txns-inputs-missingorspent")
                    {
                        return Err(e);
                    }
                    sleep(Duration::from_secs(10)).await;
                }
            }
        }
    }

    pub async fn build_and_sign(&self, anchor_info: types::AnchorInfo) -> Result<Transaction> {
        let (anchor_tx, prevouts) = build_helper::build_anchor_tx(anchor_info).await?;
        let signed_tx = signer::sign_tx(self.wif.clone(), anchor_tx, prevouts, vec![0]).await?;
        info!("{}", serialize_hex(&signed_tx));
        Ok(signed_tx)
    }
}

#[cfg(test)]
mod tests {
    use super::TxSender;
    use crate::config;
    use bitcoin::{consensus::encode::deserialize_hex, OutPoint, Transaction};
    use bittx::lightning::check_lightning_channel_close;
    use datatypes::types;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_anchor_send() {
        let cfg = config::load_config("./config.toml");
        let sender = TxSender::new(&cfg).await;
        let raw_tx = "0200000000010178fe51519ed02464f9d3c09888857b9558afb155d16fc4f72aaad6870e201d750000000000dc35df80044a010000000000002200200a4e28601b900086f4cf4fa6f247bd96c535edb8a4a894636d2d5e58008c6b354a010000000000002200208a9884a0a051ba1ed3dfcec7877a8c5437f5c81e4d775ed6886d183af620b46dcff1020000000000220020780633c65fbbeb079fe4e90f6d1745403c2f4b3c9bacc06a2c1042465d98e63f99aa03000000000022002074628c124a040fbd05c99fcca60cb433bf73ceacb11f37471a18da12067db7750400473044022050441fee1326e6e4e716805dacc108ad8cad52744f480a8d9a70db2c32e4160002204f560d068bad1df69580280d3f60be7a758ff5efd20806e7846cc016142ec02b01483045022100a91cba623b9bbc985be3e781c1cdd196d9c42db86bbb923394b3dd057327c97e02202a344d347a05c785dbb5022906520f6046f9b7264370769380627774026a8927014752210223fad034950098b0cedf25b5cdcff13540c47fb288c51650c74200bffb4fa6502103079763bb5b9d7832783e680d4f1cacd8ba95abaf8bfdb5eb9d49ba8abe5782db52aed7998a20";
        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let unlock_info = check_lightning_channel_close(&tx).unwrap();
        let anchor_info = types::AnchorInfo {
            anchor_txid: tx.compute_txid().to_string(),
            unlock_bytes: vec![unlock_info.unlock1, unlock_info.unlock2],
            unlock_outs: vec![
                (
                    tx.output.first().unwrap().clone(),
                    OutPoint {
                        txid: tx.compute_txid(),
                        vout: 0,
                    },
                ),
                (
                    tx.output.get(1).unwrap().clone(),
                    OutPoint {
                        txid: tx.compute_txid(),
                        vout: 1,
                    },
                ),
            ],
            recipient: cfg.sign.receiver.clone(),
        };
        let res = sender.build_and_sign(anchor_info).await;
        assert!(res.is_ok());

        let res1 = sender
            .send_task_by_hash(tx.compute_txid().to_string())
            .await;
        assert!(res1.is_ok());
    }
}
