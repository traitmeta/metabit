use super::*;

#[derive(Debug, PartialEq, Default, FromRow)]
pub struct AnchorTx {
    pub tx_id: String,
    pub tx_hex: String,
    pub confirmed_block_height: u64,
    pub timestamp: i64,
}

#[derive(Debug, PartialEq, Default, FromRow)]
pub struct AnchorTxOut {
    pub tx_id: String,
    pub vout: i32,
    pub value: i64,
    pub script_pubkey: String,
    pub unlock_info: String,
    pub spent: bool,
    pub confirmed_block_height: i64,
}
