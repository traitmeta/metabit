use crate::btcrpc;
use anyhow::Result;
use bitcoin::{Transaction, Txid};

pub struct TxSender {
    rpc: btcrpc::BtcCli,
}

impl TxSender {
    pub fn new(rpc: btcrpc::BtcCli) -> Self {
        Self { rpc }
    }

    pub fn send(&self, tx: Transaction) -> Result<Txid> {
        self.rpc.send_tx(&tx)
    }
}
