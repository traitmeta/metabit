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

        self.send(signed_tx)
    }
}
