use super::*;
#[derive(Debug, PartialEq, Default, FromRow)]
pub struct Indexer {
    pub height: i64,
    pub hash: String,
    pub chain_name: String,
}
