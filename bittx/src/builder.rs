use super::*;

use crate::vsize::get_tx_vsize;

pub fn build_transfer_tx(
    sender: &str,
    receiver: &str,
    amount: u64,
    in_utxos: Vec<types::Utxo>,
) -> Transaction {
    let sender_address = Address::from_str(&sender)
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

    let recipient_address = Address::from_str(&receiver)
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

    build_tx(inputs, outputs, 4.0)
}

pub fn build_tx(inputs: Vec<types::Utxo>, mut outputs: Vec<TxOut>, fee_rate: f32) -> Transaction {
    let mut tx_ins = vec![];
    for input in inputs.iter() {
        let tx_in = TxIn {
            previous_output: input.out_point,
            script_sig: ScriptBuf::new(),
            sequence: Sequence(0xffffffff),
            witness: Witness::new(),
        };

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

    tx
}

fn calc_change_amount(inputs: Vec<types::Utxo>, outputs: &Vec<TxOut>, fee_rate: f32) -> Amount {
    let mut tx_ins = vec![];
    let mut input_val: u64 = 0;
    for input in inputs.iter() {
        let tx_in = TxIn {
            previous_output: input.out_point,
            script_sig: ScriptBuf::new(),
            sequence: Sequence(0xffffffff),
            witness: Witness::new(),
        };

        input_val += input.value.to_sat();
        tx_ins.push(tx_in);
    }

    let output_val = outputs.iter().map(|out| out.value).sum::<Amount>();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::from_time(0).unwrap(),
        input: tx_ins,
        output: outputs.clone(),
    };

    let vsize = get_tx_vsize(tx);
    let fee = fee_rate * vsize as f32;
    let change_amount = input_val - output_val.to_sat() - fee as u64;
    Amount::from_sat(change_amount)
}
