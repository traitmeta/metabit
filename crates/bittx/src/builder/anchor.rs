use bitcoin::{
    opcodes::all::{OP_CSV, OP_ENDIF, OP_IFDUP, OP_NOTIF, OP_PUSHBYTES_33, OP_PUSHNUM_16},
    script::Builder,
};
use std::vec;

use super::*;

/// need a UTXO which will be added
pub fn build_lightning_anchor_tx(
    adder_utxos: &types::Utxo,
    anchor_utxos: Vec<types::Utxo>,
    input_payloads: Vec<Vec<u8>>,
) -> (Transaction, Vec<TxOut>) {
    let recipient_amount = Amount::from_sat(adder_utxos.value.to_sat() + 100);
    let receiver_out = TxOut {
        value: recipient_amount,
        script_pubkey: adder_utxos.script_pubkey.clone(),
    };

    let outputs: Vec<TxOut> = vec![receiver_out];
    let (witness_inputs, prev_fetcher) =
        build_anchor_input_and_prev_fetch(adder_utxos, anchor_utxos, input_payloads);
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: witness_inputs,
        output: outputs,
    };

    (tx, prev_fetcher)
}

pub fn build_anchor_input_and_prev_fetch(
    adder_utxo: &types::Utxo,
    inputs: Vec<types::Utxo>,
    input_payloads: Vec<Vec<u8>>,
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

    for (idx, input) in inputs.iter().enumerate() {
        let witness = build_anchor_witness(input_payloads.get(idx).unwrap());
        let tx_in = TxIn {
            previous_output: input.out_point,
            script_sig: ScriptBuf::new(),
            sequence: Sequence(0x10),
            witness,
        };
        prevouts.push(TxOut {
            value: input.value,
            script_pubkey: input.script_pubkey.clone(),
        });
        tx_ins.push(tx_in);
    }

    (tx_ins, prevouts)
}

/// OP_PUSHBYTES_33 [payload]
/// OP_CHECKSIG
/// OP_IFDUP
/// OP_NOTIF
/// OP_PUSHNUM_16
/// OP_CSV
/// OP_ENDIF
pub fn build_anchor_witness(payload: &Vec<u8>) -> Witness {
    let mut witness = Witness::new();
    witness.push(Vec::new());

    let payload: &PushBytes = payload.as_slice().try_into().unwrap();
    let script = Builder::new()
        .push_opcode(OP_PUSHBYTES_33)
        .push_slice(payload)
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_IFDUP)
        .push_opcode(OP_NOTIF)
        .push_opcode(OP_PUSHNUM_16)
        .push_opcode(OP_CSV)
        .push_opcode(OP_ENDIF)
        .into_script();

    witness.push(script);

    witness
}
