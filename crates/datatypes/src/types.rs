use bitcoin::{Amount, OutPoint, ScriptBuf, Transaction, TxOut};

#[derive(Default, Debug, Clone)]
pub struct Utxo {
    pub out_point: OutPoint,
    pub value: Amount,
    pub script_pubkey: ScriptBuf,
}

#[derive(Clone, Debug)]
pub struct TransferInfo {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub feerate: f32,
}

#[derive(Clone, Debug)]
pub struct AnchorInfo {
    pub anchor_txid: String,
    pub unlock_bytes: Vec<Vec<u8>>,
    pub unlock_outs: Vec<(TxOut, OutPoint)>,
    pub recipient: String,
}

#[derive(Clone, Debug)]
pub struct AnchorsInfo {
    pub details: Vec<AnchorDetail>,
    pub recipient: String,
}

#[derive(Clone, Debug)]
pub struct AnchorDetail {
    pub anchor_txid: String,
    pub vout: u32,
    pub redeem_script_hex: String,
    pub script_pubkey_hex: String,
    pub out_value: u64,
}

#[derive(Clone, Debug)]
pub struct AnchorUnlockInfo {
    pub unlock1: Vec<u8>,
    pub unlock2: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct AnchorUnlockDetail {
    pub vout: u32,
    pub redeem_script_hex: String,
    pub out_value: u64,
    pub nsequence: u64,
}

#[derive(Clone, Debug)]
pub struct UnsignedInfo {
    pub tx: Transaction,
    pub input_idx: u32,
    pub input_out: TxOut,
    pub recipient: String,
}
