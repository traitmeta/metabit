use datatypes::types::AnchorUnlockInfo;

use super::*;

pub struct LightningChecker {}

impl LightningChecker {
    pub fn new() -> Self {
        LightningChecker {}
    }

    pub fn check_input_sign(&self, tx: &Transaction) -> Option<AnchorUnlockInfo> {
        bittx::lightning::check_lightning_channel_close(tx)
    }
}
