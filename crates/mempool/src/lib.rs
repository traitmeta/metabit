pub mod utxo;
pub mod tx;

use anyhow::Result;
use bitcoin::{Amount, OutPoint};
use serde::Deserialize;
use tracing::debug;

const MEMPOOL_URL: &str = "https://mempool.space";

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
