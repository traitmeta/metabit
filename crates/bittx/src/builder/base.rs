use super::*;
use std::vec;

pub fn build_transfer_tx(
    sender: &str,
    receiver: &str,
    amount: u64,
    fee_rate: f32,
    in_utxos: Vec<types::Utxo>,
) -> (Transaction, Vec<TxOut>) {
    let sender_address = Address::from_str(sender)
        .unwrap()
        .require_network(Network::Bitcoin)
        .unwrap();
    let mut inputs = vec![];
    for utxo in in_utxos.iter() {
        let txin = types::Utxo {
            out_point: utxo.out_point,
            script_pubkey: sender_address.script_pubkey(),
            value: utxo.value,
        };
        inputs.push(txin);
    }

    let recipient_address = Address::from_str(receiver)
        .unwrap()
        .require_network(Network::Bitcoin)
        .unwrap();
    let recipient_amount = Amount::from_sat(amount);
    // 创建交易输出
    let receiver_out = TxOut {
        value: recipient_amount,
        script_pubkey: recipient_address.script_pubkey(),
    };

    let change_out = TxOut {
        value: Amount::from_sat(0),
        script_pubkey: sender_address.script_pubkey(),
    };
    let outputs = vec![receiver_out, change_out];

    build_tx(inputs, outputs, fee_rate)
}

pub fn build_tx(
    inputs: Vec<types::Utxo>,
    mut outputs: Vec<TxOut>,
    fee_rate: f32,
) -> (Transaction, Vec<TxOut>) {
    let mut tx_ins = vec![];
    let mut prevouts = Vec::new();

    for input in inputs.iter() {
        let tx_in = TxIn {
            previous_output: input.out_point,
            script_sig: ScriptBuf::new(),
            sequence: Sequence(0xffffffff),
            witness: Witness::new(),
        };
        prevouts.push(TxOut {
            value: input.value,
            script_pubkey: input.script_pubkey.clone(),
        });
        tx_ins.push(tx_in);
    }

    let change_amount = calc_change_amount(inputs, &outputs, fee_rate);
    let mut change_output = outputs.pop().unwrap();
    change_output.value = change_amount;
    outputs.push(change_output);
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::from_time(0).unwrap(),
        input: tx_ins,
        output: outputs,
    };

    (tx, prevouts)
}

fn calc_change_amount(inputs: Vec<types::Utxo>, outputs: &[TxOut], fee_rate: f32) -> Amount {
    let mut tx_ins = vec![];
    let mut input_val: u64 = 0;
    for input in inputs.iter() {
        let tx_in = TxIn {
            previous_output: input.out_point,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::from_slice(&[&[0; SCHNORR_SIGNATURE_SIZE]]),
        };

        input_val += input.value.to_sat();
        tx_ins.push(tx_in);
    }

    let output_val = outputs.iter().map(|out| out.value).sum::<Amount>();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::from_time(0).unwrap(),
        input: tx_ins,
        output: outputs.to_owned(),
    };

    // let vsize = get_tx_vsize(tx);
    let vsize = tx.vsize();
    let fee = fee_rate * vsize as f32;
    let change_amount = input_val - output_val.to_sat() - fee as u64;
    Amount::from_sat(change_amount)
}
