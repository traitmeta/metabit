use bitcoin::blockdata::transaction::Transaction;
use bitcoin::consensus::encode::serialize;

pub fn get_tx_vsize(tx: Transaction) -> usize {
    let non_witness_size = serialize(&tx).len();
    let witness_size = tx
        .input
        .iter()
        .map(|input| input.witness.len())
        .sum::<usize>();

    let weight = 3 * non_witness_size + witness_size;
    let vsize = (weight + 3) / 4;

    vsize
}
