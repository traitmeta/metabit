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

    pub async fn send_unsigned_tx(
        &self,
        tx: Transaction,
        idx: u32,
        my_utxos: Vec<types::Utxo>,
    ) -> Result<()> {
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

        if my_utxos.is_empty() {
            return Err(anyhow!("not found unspent utxo"));
        }

        let my_utxo = my_utxos.first().unwrap();
        info!("start build unsign_tx...");
        // match build_helper::build_unsigned_tx(info).await {
        match build_helper::build_unsigned_tx_with_receive_utxo(info, my_utxo.clone()).await {
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

    pub async fn send_task(&self, my_utxos: Vec<types::Utxo>) -> Result<()> {
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
            match self.build_and_sign(anchor_info, &my_utxos).await {
                Ok(tx) => {
                    info!("send transaction started: {:?}", tx.compute_txid());
                    self.send(tx)?;
                }
                Err(e) => return Err(anyhow!("build and sign tx fail : {}", e)),
            };
        }
        Ok(())
    }

    pub async fn send_task_by_hash(
        &self,
        tx_id: String,
        my_utxos: Vec<types::Utxo>,
    ) -> Result<Transaction> {
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

        self.build_and_sign(anchor_info, &my_utxos).await
    }

    pub async fn build_sign_and_send(
        &self,
        anchor_info: types::AnchorInfo,
        my_utxos: Vec<types::Utxo>,
    ) -> Result<Txid> {
        let my_utxo = my_utxos.first().unwrap();
        let (anchor_tx, prevouts) =
            build_helper::build_anchor_tx(anchor_info, my_utxo.clone()).await?;
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

    pub async fn build_and_sign(
        &self,
        anchor_info: types::AnchorInfo,
        my_utxos: &[types::Utxo],
    ) -> Result<Transaction> {
        let my_utxo = my_utxos.first().unwrap();
        let (anchor_tx, prevouts) =
            build_helper::build_anchor_tx(anchor_info, my_utxo.clone()).await?;
        let signed_tx = signer::sign_tx(self.wif.clone(), anchor_tx, prevouts, vec![0]).await?;
        info!("{}", serialize_hex(&signed_tx));
        Ok(signed_tx)
    }
}

#[cfg(test)]
mod tests {
    use super::TxSender;
    use crate::{btcrpc, config};
    use bitcoin::{
        consensus::encode::{deserialize_hex, serialize_hex},
        key::Secp256k1,
        taproot::TaprootSpendInfo,
        Address, Network, OutPoint, PrivateKey, Transaction,
    };
    use bittx::{build_helper, lightning::check_lightning_channel_close, signer};
    use datatypes::types;
    use tracing::info;

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
        let my_utxos = vec![];
        let res = sender.build_and_sign(anchor_info, &my_utxos).await;
        assert!(res.is_ok());

        let res1 = sender
            .send_task_by_hash(tx.compute_txid().to_string(), my_utxos)
            .await;
        assert!(res1.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_unsigned_tx() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .try_init();

        let cfg = config::load_config("./config.toml");
        let sender = TxSender::new(&cfg).await;
        let raw_tx = "0200000000010264f41669722e08cc5c8c75a7d94fd8658889e18db5903a0b86c015832a4ca2360000000000ffffffff8f2b243127e5c00ec4de5b71ec33db6e2aabad29c1828b91b60722bc2ceaf91f0000000000ffffffff01f30500000000000016001492b8c3a56fac121ddcdffbc85b02fb9ef681038a0247304402201090f8622eb31b7a6e79afcf2fc38eaf767b703779430e229dc8d3faca2e4f190220134db78f6840cab74ca4d113332a3812ffdf5b39813c983f826dbdb5e99f1b110121030c7196376bc1df61b6da6ee711868fd30e370dd273332bfb02a2287d11e2e9c5030101fdef0251690063036f7264010117746578742f68746d6c3b636861727365743d7574662d38004d08023c73637269707420646174612d733d2230783738623766363166396431643731313263616532626434326161363235633433326134306532346165646538623237323963346365656433336432383065613922207372633d222f636f6e74656e742f663830623933343636613238633565666337303366616230326265656262663465333265316263346630363361633237666564666437396164393832663263656930223e3c2f7363726970743e3c626f6479207374796c653d22646973706c61793a206e6f6e65223e3c2f626f64793e00000000000000004f505f42564d5f56321b5f02e82f076c70afcc81987b472fd4a347ec4fa4a74da99f9d0c19a1d2ffff71af54e29a1f98c654ee7d38671960a96b6cdb4f03d000e62d418df013a0933521404a9356d87f625c3ee86dd54a809c64365f871b41c1e391075cac2e9127e6b5b4292e94c1500320fc2c19e6dc6b4b0fa83fc8ffaea3c4d7653f696162e293a4f96733b043d9e08cd30a404f596955ff94b4aa3b1356a092b7ef407509639c32aa4b8294918146c62122618e09a684074f1d8680c88321a08408da02370a697046c4880ebcb4d682e9eeeb6b0b8f1963aa79ad5b67617bd3c6a24edba3aaadf2db6ad967c2eac896b3206dd47bf1949af7466c24a65f4fa2bb4fddeff99787ce92bdd3766fa54759196e59f2f00fc14989325c791a14f192f38008e8918ae888200834124cbeefa946e99d03156390c23b894e2bca62b05251deddd7dfa6ea57b0f3bef3bae1183aa0b72a96bca37ea752599f838b8ab9121feade6a0bb8d77bfbb78ff5527e4785d99f1bd99f5c687e7769160d6fada3aa401eb1346725d246eb2dd7da71e3988720a403ca08c410a08c5b2fc153f486714b0586408d899418f49a681ee3fd7212bde65427e074384ef32f1b3e53ab3bf8a09910a6f7b7146cbe7dde0fc0c9596b17a3db4a26bfca2796316f43a6c29512235163c3e5cf7b0bfdbb1e006821c08c6be8f1a0311e1bf2e3f3b997397a2f08655f37d8ef1ccd2b57dfbde917d83300000000";
        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let my_utxo = types::Utxo {
            out_point: OutPoint {
                txid: tx.compute_txid(),
                vout: 0,
            },
            value: tx.output[0].value,
            script_pubkey: tx.output[0].script_pubkey.clone(),
        };

        let res1 = sender.send_unsigned_tx(tx, 1, vec![my_utxo]).await;
        assert!(res1.is_err());
        println!("{:?}", res1.unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_send_tx() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .try_init();

        let btcrpc = btcrpc::BtcCli::new("127.0.0.1:18443", "meta", "meta");
        let regtest_wif = "KwHui7RBgSXNAvXPMhU66VvAisjRjDrEarrXbgHsyg1VqmkHEUbd";
        let private_key = PrivateKey::from_wif(regtest_wif).unwrap();
        let secp = Secp256k1::new();
        let taproot_spend_info = TaprootSpendInfo::new_key_spend(
            &secp,
            private_key.public_key(&secp).inner.into(),
            None,
        );

        let address = Address::p2tr(
            &secp,
            private_key.public_key(&secp).inner.into(),
            None,
            Network::Regtest,
        );

        info!("address: {}", address);
        let tweaked_address =
            Address::p2tr_tweaked(taproot_spend_info.output_key(), Network::Regtest);
        info!("tweaked_address: {}", tweaked_address);

        let my_tx = "020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff025100ffffffff0200f2052a0100000022512042a95db764d2c9cffad226fd40e0c1f181f3b71fa1aef78c25d49998836851540000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000";
        let tx = deserialize_hex::<Transaction>(&my_tx).unwrap();
        let my_utxo = types::Utxo {
            out_point: OutPoint {
                txid: tx.compute_txid(),
                vout: 0,
            },
            value: tx.output[0].value,
            script_pubkey: tx.output[0].script_pubkey.clone(),
        };

        // let cfg = config::load_config("./config.toml");
        // let sender = TxSender::new(&cfg).await;
        // let raw_tx = "0200000000010264f41669722e08cc5c8c75a7d94fd8658889e18db5903a0b86c015832a4ca2360000000000ffffffff8f2b243127e5c00ec4de5b71ec33db6e2aabad29c1828b91b60722bc2ceaf91f0000000000ffffffff01f30500000000000016001492b8c3a56fac121ddcdffbc85b02fb9ef681038a0247304402201090f8622eb31b7a6e79afcf2fc38eaf767b703779430e229dc8d3faca2e4f190220134db78f6840cab74ca4d113332a3812ffdf5b39813c983f826dbdb5e99f1b110121030c7196376bc1df61b6da6ee711868fd30e370dd273332bfb02a2287d11e2e9c5030101fdef0251690063036f7264010117746578742f68746d6c3b636861727365743d7574662d38004d08023c73637269707420646174612d733d2230783738623766363166396431643731313263616532626434326161363235633433326134306532346165646538623237323963346365656433336432383065613922207372633d222f636f6e74656e742f663830623933343636613238633565666337303366616230326265656262663465333265316263346630363361633237666564666437396164393832663263656930223e3c2f7363726970743e3c626f6479207374796c653d22646973706c61793a206e6f6e65223e3c2f626f64793e00000000000000004f505f42564d5f56321b5f02e82f076c70afcc81987b472fd4a347ec4fa4a74da99f9d0c19a1d2ffff71af54e29a1f98c654ee7d38671960a96b6cdb4f03d000e62d418df013a0933521404a9356d87f625c3ee86dd54a809c64365f871b41c1e391075cac2e9127e6b5b4292e94c1500320fc2c19e6dc6b4b0fa83fc8ffaea3c4d7653f696162e293a4f96733b043d9e08cd30a404f596955ff94b4aa3b1356a092b7ef407509639c32aa4b8294918146c62122618e09a684074f1d8680c88321a08408da02370a697046c4880ebcb4d682e9eeeb6b0b8f1963aa79ad5b67617bd3c6a24edba3aaadf2db6ad967c2eac896b3206dd47bf1949af7466c24a65f4fa2bb4fddeff99787ce92bdd3766fa54759196e59f2f00fc14989325c791a14f192f38008e8918ae888200834124cbeefa946e99d03156390c23b894e2bca62b05251deddd7dfa6ea57b0f3bef3bae1183aa0b72a96bca37ea752599f838b8ab9121feade6a0bb8d77bfbb78ff5527e4785d99f1bd99f5c687e7769160d6fada3aa401eb1346725d246eb2dd7da71e3988720a403ca08c410a08c5b2fc153f486714b0586408d899418f49a681ee3fd7212bde65427e074384ef32f1b3e53ab3bf8a09910a6f7b7146cbe7dde0fc0c9596b17a3db4a26bfca2796316f43a6c29512235163c3e5cf7b0bfdbb1e006821c08c6be8f1a0311e1bf2e3f3b997397a2f08655f37d8ef1ccd2b57dfbde917d83300000000";
        // let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        // let res1 = sender.send_unsigned_tx(tx, 1).await;
        let transfer_info = types::TransferInfo {
            sender: "bcrt1pg254mdmy6tyul7kjym75pcxp7xql8dcl5xh00rp96jve3qmg292qju447x".to_string(),
            recipient: "bcrt1q8g8nly0syz3kksgtvdymae0xlgxnawvyrhc4pf".to_string(),
            amount: 100000000,
            feerate: 3.0,
        };

        let (unsigned_tx, prev_outs) = build_helper::build_transer_tx_with_utxo(
            transfer_info,
            vec![my_utxo],
            Some(Network::Regtest),
        )
        .await
        .unwrap();

        match signer::sign_tx(regtest_wif.to_string(), unsigned_tx, prev_outs, vec![0]).await {
            Ok(signed_tx) => {
                info!(
                    "build and signed the transfer tx, id: {} hex : {}",
                    signed_tx.compute_txid(),
                    serialize_hex(&signed_tx)
                );
                btcrpc.send_tx(&signed_tx).unwrap();
            }
            Err(err) => {
                info!("failed to sign the unsign_tx: {:?}", err);
            }
        }
        // assert!(res1.is_err());
        // println!("{:?}", res1.unwrap());
    }
}
