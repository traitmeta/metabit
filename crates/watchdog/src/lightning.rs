const SWEPT_LIGHTNING_ANCHOR: &str =
    "21027aa14599b7b2fc79a0996f6d1c9f739436e4724a2ea72ca806416000794991dfac736460b268";

pub fn is_swept_lightning_anchor(data: &str) -> bool {
    data == SWEPT_LIGHTNING_ANCHOR
}
