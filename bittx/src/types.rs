use bitcoin::{Amount, OutPoint, ScriptBuf};

pub struct Utxo {
    pub out_point: OutPoint,
    pub value: Amount,
    pub script_pubkey: ScriptBuf,
}

pub struct TransferInfo {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
}
