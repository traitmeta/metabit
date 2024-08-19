// TODO some wrong. may be not identify with this
const SWEPT_LIGHTNING_ANCHOR: &str =
    "21027aa14599b7b2fc79a0996f6d1c9f739436e4724a2ea72ca806416000794991dfac736460b268";

pub fn is_swept_lightning_anchor(data: &str) -> bool {
    data == SWEPT_LIGHTNING_ANCHOR
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::blockdata::transaction::Transaction;
    use bitcoin::consensus::encode::deserialize_hex;

    #[test]
    fn is_swept_lightning_anchor_test() {
        let raw_tx = "02000000000102b286d5acc329aecf0473d19973d9f53dcf42f11e19b9057b1fdaf48083d6c9fd010000000010000000fbf3b729fdf5d0df9c9af156652e4f68cb9b81b9197b9573fd2f87e9e6e1208a010000000010000000010000000000000000036a010002002821029b97027605f86846ac84edbba3913e5e0e164c2639d6f336d401237c09ccae7fac736460b26802002821028a468e95c1899be5b9838f10595e494d7fa932b600e84d58df9f441ada3cba29ac736460b26800000000";

        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let input_0 = tx.input.get(0).unwrap();
        let input_0_wintess_1_hex = hex::encode(&input_0.witness[1]);
        println!("{}", input_0_wintess_1_hex);
        let res = is_swept_lightning_anchor(&input_0_wintess_1_hex);
        assert!(res);
    }
}
