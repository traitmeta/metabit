use bitcoin::{Amount, OutPoint, ScriptBuf};

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
