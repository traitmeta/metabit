use bitcoin::{Amount, OutPoint, ScriptBuf};

pub struct Utxo {
    pub out_point: OutPoint,
    pub value: Amount,
    pub script_pubkey: ScriptBuf,
}
