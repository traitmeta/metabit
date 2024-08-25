use bitcoin::consensus::encode::serialize_hex;
use bittx::{build_helper, signer};
use datatypes::types;

use super::*;

pub struct TxSender {
    rpc: btcrpc::BtcCli,
    receiver: String,
    wif: String,
}

impl TxSender {
    pub fn new(rpc: btcrpc::BtcCli, receiver: String, wif: String) -> Self {
        Self { rpc, receiver, wif }
    }

    pub fn send(&self, tx: Transaction) -> Result<Txid> {
        self.rpc.send_tx(&tx)
    }

    pub async fn build_sign_and_send(
        &self,
        tx: Transaction,
        unlock_info: types::AnchorUnlockInfo,
    ) -> Result<Txid> {
        let anchor_info = types::AnchorInfo {
            anchor_txid: tx.compute_txid().to_string(),
            unlock_bytes: vec![unlock_info.unlock1, unlock_info.unlock2],
            unlock_outs: vec![
                tx.output.get(0).unwrap().clone(),
                tx.output.get(1).unwrap().clone(),
            ],
            recipient: self.receiver.clone(),
        };

        let anchor_tx = build_helper::build_anchor_tx(anchor_info).await?;
        let signed_tx =
            signer::sign_tx(self.wif.clone(), anchor_tx.0, anchor_tx.1, vec![0]).await?;
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
}

#[cfg(test)]
mod tests {
    use bitcoin::{
        consensus::encode::{deserialize_hex, serialize_hex},
        Transaction,
    };
    use bittx::{build_helper::build_anchor_tx, lightning::check_lightning_channel_close};
    use datatypes::types;

    use crate::{btcrpc::BtcCli, config};

    use super::{btcrpc, receiver, TxSender};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_anchor_send() {
        let cfg = config::load_config("./config.toml");
        let btc_cli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
        let sender = TxSender::new(
            btc_cli,
            cfg.sign.receiver.to_owned(),
            cfg.sign.wif.to_owned(),
        );

        let raw_tx = "02000000000101caae31cea641ed5b9d93a2764fa25cee5910e2d52de46a11b8d0ae72da05810a0000000000268e2280044a01000000000000220020286b5328b8210524ac31db6be20008f7f5ee5e8ae78ef12be3b3d575effe2a4c4a010000000000002200203cdcdf9c59ea871d62eb671f3b3dda139e2fd657b68e3a21da83d0df368b57b75509010000000000220020f2824d1fd3ddfb51e5f367c53d9c5937a862aa7c5dfae9259885dd166c4b52507687010000000000220020b5469e22001cdb3845ef6cd6e153b025371db3a0597072725a1ebe492e9104ed0400473044022036ab696ccfc27b8864ac37471910e272d6fa2f5b96f83812e4ba1e89189bcb6502206154ec8d7db30799dd448784088bdfae83dc6bb97f74db1e363494ecc5638d920147304402204f4d2d8c62ee4d6ac75ec99bc0a3d110553cdb86179c914b1149123d4da8f2d902201f2b29260df0d53a1542b9c8ef8b7b7f6c4ca365a5082c887668955539ff222501475221025f1432932c9ba37ef4fd060d9f14050325fe433cd2ea49b636797a6cbec80b082102e96bfcb258f0ae9530802fea157137123e0f2f9da70119d0a25822e1ee5f398b52aea6a96b20";
        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();

        let unlock_info = check_lightning_channel_close(&tx).unwrap();
        let res = sender.build_sign_and_send(tx, unlock_info).await;
        assert!(res.is_err())
    }
}
