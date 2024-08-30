use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    Amount, OutPoint, ScriptBuf,
};
use bittx::{build_helper, signer};
use btcrpc::BtcCli;
use datatypes::types;
use std::{collections::HashMap, str::FromStr};

use super::*;

#[derive(Debug)]
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

    pub async fn send_unsigned_tx(&self, tx: Transaction, idx: u32) -> Result<()> {
        let input = tx.input.get(idx as usize).unwrap();
        let prev_out = self
            .btccli
            .get_tx_out(&input.previous_output.txid, input.previous_output.vout)?;
        let info = types::UnsignedInfo {
            recipient: self.receiver.clone(),
            tx,
            input_idx: idx,
            input_out: prev_out,
        };

        info!("start build unsign_tx...");
        let my_tx = "02000000000101067acde4371345825c3af8e105ce9a17435df98462a56a62ebed810ab621d7ed0000000000ffffffff02e803000000000000225120e674888588bef4c0abaf7027fe17f0afcf1a6f71d8d5603ff8dbb423868154bb801f110000000000225120f607175c05ffe3a3143190924f940b20fb1e19c3e2ceca2c743f8a14a69f09900140fdf2faae2f9e0b8d8e93502271eca49171e0263c510e4a425bd238bb49ce4578361684d8dcfadc7a71ea0d813fb37b4b48a4efb098dcc61e07b21c614cc7e98e00000000";
        let tx = deserialize_hex::<Transaction>(&my_tx).unwrap();
        let my_utxo = types::Utxo {
            out_point: OutPoint {
                txid: tx.compute_txid(),
                vout: 0,
            },
            value: tx.output[0].value,
            script_pubkey: tx.output[0].script_pubkey.clone(),
        };
        // match build_helper::build_unsigned_tx(info).await {
        match build_helper::build_unsigned_tx_with_receive_utxo(info, my_utxo).await {
            Ok((unsigned_tx, prevouts)) => {
                match signer::sign_tx(self.wif.clone(), unsigned_tx, prevouts, vec![0]).await {
                    Ok(signed_tx) => {
                        info!(
                            "build and signed the unsign_tx, id: {} hex : {}",
                            signed_tx.compute_txid(),
                            serialize_hex(&signed_tx)
                        );
                        self.send(signed_tx.clone())?;
                    }
                    Err(err) => {
                        error!("failed to sign the unsign_tx: {:?}", err);
                        return Err(err);
                    }
                }
            }
            Err(err) => {
                error!("build unsign_tx error: {:?}", err);
                return Err(err);
            }
        }
        Ok(())
    }

    pub async fn send_task(&self) -> Result<()> {
        let height = self.btccli.get_best_block_height();
        if height.is_err() {
            return Err(anyhow!("get block height failed"));
        }

        debug!("send task get block height successfully");
        let height = height.unwrap();
        let txouts = self.dao.get_anchor_tx_out(height as i64).await;
        if txouts.is_err() {
            return Err(anyhow!("get anchor txouts failed"));
        }

        let txouts = txouts.unwrap();
        if txouts.len() <= 1 {
            return Ok(());
        }

        info!("send task get db successfully. len {}", txouts.len());
        let mut datas = HashMap::new();
        for tx_out in txouts.into_iter() {
            datas
                .entry(tx_out.tx_id.clone())
                .or_insert_with(Vec::new)
                .push(tx_out);
        }

        for (tx_id, tx_outs) in &datas {
            if tx_outs.len() != 2 {
                continue;
            }

            info!("send task into build. txid {}", tx_id);
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
            info!("send task into build finish. txid {}", tx_id);
            match self.build_and_sign(anchor_info).await {
                Ok(tx) => {
                    info!("send transaction started: {:?}", tx.compute_txid());
                    self.send(tx)?;
                }
                Err(e) => return Err(anyhow!("build and sign tx fail : {}", e)),
            };
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_unsigned_tx() {
        let cfg = config::load_config("./config.toml");
        let sender = TxSender::new(&cfg).await;
        let raw_tx = "020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff3103cd1b0d0456c1d1662f466f756e6472792055534120506f6f6c202364726f70676f6c642f4ef75b667b36000000000000ffffffff0322020000000000002251203daaca9b82a51aca960c1491588246029d7e0fc49e0abdbcc8fd17574be5c74b66db54130000000016001435f6de260c9f3bdee47524c473a6016c0c055cb90000000000000000266a24aa21a9edde1dc3347583bc6bcb35776b1c51ba5e2254b5c377ea464a55db029bf11240670120000000000000000000000000000000000000000000000000000000000000000000000000";
        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let res1 = sender.send_unsigned_tx(tx, 0).await;
        assert!(res1.is_err());
        println!("{:?}", res1.unwrap());
    }
}
