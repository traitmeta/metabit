use bitcoin::{consensus::encode::serialize_hex, Amount, OutPoint, ScriptBuf};
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
            .get_tx_out(&input.previous_output.txid, input.previous_output.vout)
            .unwrap();
        let info = types::UnsignedInfo {
            recipient: self.receiver.clone(),
            tx,
            input_idx: idx,
            input_out: prev_out,
        };

        info!("start build unsign_tx...");
        match build_helper::build_unsigned_tx(info).await {
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

        info!("send task get block height successfully");
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
        let raw_tx = "02000000000102cf83df52df0001b996c4a6d6082d1330d3a663fdb19aa389f7a71c84c0b761020000000000ffffffff2729ef9b7bfe38d441d84e91e5cb878c3520230a6e1d5bbb34b94cdec94b5cba0100000000ffffffff01c70800000000000016001492b8c3a56fac121ddcdffbc85b02fb9ef681038a0247304402202336d99eb756b60ebe2b3228a121ade61c40ca51480391e14e944da8c2563ff10220028c1d84851be53d4d4d83411ecbc6b89dff0a1306cd9651ca615c7f43d7a6df0121030c7196376bc1df61b6da6ee711868fd30e370dd273332bfb02a2287d11e2e9c5030101fdfd0251690063036f7264010117746578742f68746d6c3b636861727365743d7574662d38004d08023c73637269707420646174612d733d2230783137643865363839303538343232636433636332313864326430633965663238326236633039393061626265343934313436663831666334613131326265626522207372633d222f636f6e74656e742f663830623933343636613238633565666337303366616230326265656262663465333265316263346630363361633237666564666437396164393832663263656930223e3c2f7363726970743e3c626f6479207374796c653d22646973706c61793a206e6f6e65223e3c2f626f64793e00000000000000004f505f42564d5f56321b5e02e83e0d7072cdcb712c1931621ad696bb14a58fc3ab74f8309afc60164f2354fac0381da08b114402552524dbf78eb17f3408453cc4c9ed6e3b4c3bdda63c5ea79b870c093a21aa444d26f96b023cadc3069ce48106a2cbbdc486cb2e950a4dcb03bf13ec04f965a9ef76af50c7cfcb555f683738d48872c83c308f821027830ba068c48bb6739843fc29ae0645ebf65a9eec627d13d85fdbfe0900185858b3c0b0c75ec09cc8d0cc7f18a4c9e90e439078fc2ff693426b856449141d11c01855ac97315be53903840c465ae32849e74196ecd0914d2893219f2327801c132aa6d9dada5215e18bc04dbe1bb6e9c4eadf6a5bc8fadb41af4f03f4fe161a1f2a37ee532a352820731c13edeeff24d99268a9d9f27f7e3ae18e8719b9dbfb030080febd4ccc61cdf5134c5a848846271fb2a2a2d17304c700a88934ba905272484a321be5331632a84029a0c2a4435432a9d37d3ef0b8ca5ff96ecabdefc8ec1611b4c36dd98ad57aa8cd3c85b07951a9acb7ab5f8291dd2aeaaca6b96ff8c18e27ac3baa68490b3335722f806b21d2414be931a68a912961b1524108250153224c94d982321cb8b0b6a1b89882b56025078701c159a82b9d46ccad49274bcf7be7ff58b1526a24cbee915aaae88890d9308c6c116685c7927ea5a91e3ae1d21c6eacdd3b65e23f1eab1a7cc47f377901006821c06b3105c1b3dc5d468521986f1eb85e6d1e0001a63bc3eb167698b1dc1d60dfb600000000";
        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let res1 = sender.send_unsigned_tx(tx, 1).await;
        assert!(res1.is_err());
    }
}
