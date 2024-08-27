use super::*;
use std::vec;

pub fn build_unsigned_tx(
    adder_utxos: &types::Utxo,
    inputs: Vec<TxIn>,
) -> (Transaction, Vec<TxOut>) {
    let recipient_amount = Amount::from_sat(adder_utxos.value.to_sat() + 100);
    let receiver_out = TxOut {
        value: recipient_amount,
        script_pubkey: adder_utxos.script_pubkey.clone(),
    };

    let outputs: Vec<TxOut> = vec![receiver_out];
    let (witness_inputs, prev_fetcher) = build_unsigned_input_and_prev_fetch(adder_utxos, inputs);
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: witness_inputs,
        output: outputs,
    };

    (tx, prev_fetcher)
}

pub fn build_unsigned_input_and_prev_fetch(
    adder_utxo: &types::Utxo,
    mut inputs: Vec<TxIn>,
) -> (Vec<TxIn>, Vec<TxOut>) {
    let mut tx_ins = vec![];
    let mut prevouts = Vec::new();
    let tx_in: TxIn = TxIn {
        previous_output: adder_utxo.out_point,
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Witness::new(),
    };
    prevouts.push(TxOut {
        value: adder_utxo.value,
        script_pubkey: adder_utxo.script_pubkey.clone(),
    });
    tx_ins.push(tx_in);

    for input in &mut inputs {
        let mut wit = input.witness.to_vec();
        if let Some(first_witness) = wit.first_mut() {
            *first_witness = Vec::new();
        }
        input.witness = Witness::from(wit);
        tx_ins.push(input.clone());
    }

    (tx_ins, prevouts)
}
