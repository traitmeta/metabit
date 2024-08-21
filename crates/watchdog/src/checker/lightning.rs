use bittx::lightning::MultiSig2_2;

use super::*;

pub struct LightningChecker {}

impl LightningChecker {
    pub fn new() -> Self {
        LightningChecker {}
    }

    pub fn check_input_sign(&self, tx: &Transaction) -> Option<MultiSig2_2> {
        bittx::lightning::check_lightning_channel_close(tx)
    }
}
