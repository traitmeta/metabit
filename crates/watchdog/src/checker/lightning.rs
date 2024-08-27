use bitcoin::hex::{Case, DisplayHex};
use datatypes::types::AnchorUnlockInfo;
use repo::anchor::AnchorTxOut;

use super::*;

pub struct LightningChecker {}

impl LightningChecker {
    pub fn new() -> Self {
        LightningChecker {}
    }

    pub fn check_input_sign(&self, tx: &Transaction) -> Option<AnchorUnlockInfo> {
        bittx::lightning::check_lightning_channel_close(tx)
    }

    pub fn check_anchor(&self, tx: &Transaction) -> Option<Vec<AnchorTxOut>> {
        if let Some(unlock_info) = bittx::lightning::check_lightning_channel_close(tx) {
            info!("find anchor {:?}", unlock_info);
            let mut anchor_tx_outs = Vec::new();
            for i in 0..2 {
                let mut unlock = unlock_info.unlock1.clone();
                if i == 1 {
                    unlock = unlock_info.unlock2.clone();
                }

                let out = tx.output.get(i).unwrap();
                let anchor_model = AnchorTxOut {
                    tx_id: tx.compute_txid().to_string(),
                    vout: i as i32,
                    value: out.value.to_sat() as i64,
                    unlock_info: unlock.to_hex_string(Case::Lower),
                    script_pubkey: out.script_pubkey.to_hex_string(),
                    spent: false,
                    confirmed_block_height: 0,
                };
                anchor_tx_outs.push(anchor_model);
            }

            return Some(anchor_tx_outs);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use bitcoin::consensus::encode::deserialize_hex;

    use super::*;

    #[test]
    fn test_check_anchor() {
        let raw_tx = "02000000000101caae31cea641ed5b9d93a2764fa25cee5910e2d52de46a11b8d0ae72da05810a0000000000268e2280044a01000000000000220020286b5328b8210524ac31db6be20008f7f5ee5e8ae78ef12be3b3d575effe2a4c4a010000000000002200203cdcdf9c59ea871d62eb671f3b3dda139e2fd657b68e3a21da83d0df368b57b75509010000000000220020f2824d1fd3ddfb51e5f367c53d9c5937a862aa7c5dfae9259885dd166c4b52507687010000000000220020b5469e22001cdb3845ef6cd6e153b025371db3a0597072725a1ebe492e9104ed0400473044022036ab696ccfc27b8864ac37471910e272d6fa2f5b96f83812e4ba1e89189bcb6502206154ec8d7db30799dd448784088bdfae83dc6bb97f74db1e363494ecc5638d920147304402204f4d2d8c62ee4d6ac75ec99bc0a3d110553cdb86179c914b1149123d4da8f2d902201f2b29260df0d53a1542b9c8ef8b7b7f6c4ca365a5082c887668955539ff222501475221025f1432932c9ba37ef4fd060d9f14050325fe433cd2ea49b636797a6cbec80b082102e96bfcb258f0ae9530802fea157137123e0f2f9da70119d0a25822e1ee5f398b52aea6a96b20";
        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let res = LightningChecker::new().check_anchor(&tx);
        assert_eq!(true, res.is_some());
        println!("{:?}", res);
    }
}
