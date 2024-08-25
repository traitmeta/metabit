use bitcoin::{Amount, OutPoint, ScriptBuf, TxOut};

#[derive(Default, Debug)]
pub struct Utxo {
    pub out_point: OutPoint,
    pub value: Amount,
    pub script_pubkey: ScriptBuf,
}

pub struct TransferInfo {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub feerate: f32,
}

pub struct AnchorInfo {
    pub anchor_txid: String,
    pub unlock_bytes: Vec<Vec<u8>>,
    pub unlock_outs: Vec<TxOut>,
    pub recipient: String,
}

pub struct AnchorUnlockInfo {
    pub unlock1: Vec<u8>,
    pub unlock2: Vec<u8>,
}
