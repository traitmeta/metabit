use bitcoin::{
    opcodes::all::{OP_CSV, OP_ENDIF, OP_IFDUP, OP_NOTIF, OP_PUSHBYTES_33, OP_PUSHNUM_16},
    script::Builder,
    OutPoint,
};
use std::vec;

use super::*;

pub fn build_anchor_sweep_tx(
    my_utxo: &types::Utxo,
    anchor_details: Vec<types::AnchorDetail>,
) -> Result<(Transaction, Vec<TxOut>)> {
    if anchor_details.is_empty() {
        return Err(anyhow!(
            "build anchor swept transaction anchor_details is empty"
        ));
    }
    let recipient_amount = Amount::from_sat(my_utxo.value.to_sat() + 100);
    let receiver_out = TxOut {
        value: recipient_amount,
        script_pubkey: my_utxo.script_pubkey.clone(),
    };

    let mut tx_ins = vec![];
    let mut prevouts = Vec::new();
    let tx_in: TxIn = TxIn {
        previous_output: my_utxo.out_point,
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Witness::new(),
    };
    prevouts.push(TxOut {
        value: my_utxo.value,
        script_pubkey: my_utxo.script_pubkey.clone(),
    });
    tx_ins.push(tx_in);

    let outputs: Vec<TxOut> = vec![receiver_out];
    match builds_input_and_prev_fetch(&mut tx_ins, &mut prevouts, anchor_details) {
        Ok(_) => {
            if tx_ins.len() < 3 {
                return Err(anyhow!(
                    "build anchor swept transaction input is too less than {}",
                    tx_ins.len()
                ));
            }

            let mut tx = Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: tx_ins,
                output: outputs,
            };
            tx.output[0].value =
                Amount::from_sat(my_utxo.value.to_sat() + (tx.vsize() as u64 - 100));

            Ok((tx, prevouts))
        }
        Err(e) => Err(anyhow!("build anchor swept transaction failed: {:?}", e)),
    }
}

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

pub fn builds_input_and_prev_fetch(
    inputs: &mut Vec<TxIn>,
    prev_outs: &mut Vec<TxOut>,
    details: Vec<types::AnchorDetail>,
) -> Result<()> {
    // put anchor inputs
    for detail in details.iter() {
        let mut witness = Witness::new();
        witness.push(Vec::new());
        witness.push(ScriptBuf::from_hex(&detail.redeem_script_hex)?);

        let tx_in = TxIn {
            previous_output: OutPoint::from_str(&format!(
                "{}:{}",
                detail.anchor_txid, detail.vout
            ))?,
            script_sig: ScriptBuf::new(),
            sequence: Sequence(0x10),
            witness,
        };
        prev_outs.push(TxOut {
            value: Amount::from_sat(detail.out_value),
            script_pubkey: ScriptBuf::from_hex(&detail.script_pubkey_hex)?,
        });
        inputs.push(tx_in);
    }

    Ok(())
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

pub fn build_anchor_redeem_script(payload: &Vec<u8>) -> ScriptBuf {
    let payload: &PushBytes = payload.as_slice().try_into().unwrap();
    Builder::new()
        .push_opcode(OP_PUSHBYTES_33)
        .push_slice(payload)
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_IFDUP)
        .push_opcode(OP_NOTIF)
        .push_opcode(OP_PUSHNUM_16)
        .push_opcode(OP_CSV)
        .push_opcode(OP_ENDIF)
        .into_script()
}

pub fn calc_script_pubkey(wit: Witness) -> Result<ScriptBuf> {
    let redeem_script = wit.last().unwrap();
    let script_pubkey = ScriptBuf::from_bytes(redeem_script.to_vec()).to_p2wsh();
    Ok(script_pubkey)
}
