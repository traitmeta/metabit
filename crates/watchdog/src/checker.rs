use bitcoin::blockdata::opcodes::all::{
    OP_CHECKMULTISIG, OP_CHECKMULTISIGVERIFY, OP_CHECKSIG, OP_CHECKSIGVERIFY,
};
use bitcoin::script::Instruction;
use bitcoin::{Script, Transaction, TxIn, Txid, Witness};
use bittx::witness;

use crate::btcrpc;

pub fn is_multisig_witness(witness: &Witness) -> bool {
    if let Some(redeem_script_bytes) = witness.last() {
        let redeem_script = Script::from_bytes(redeem_script_bytes);
        redeem_script
            .instructions()
            .any(|instruction| match instruction {
                Ok(opcode) => {
                    opcode == Instruction::Op(OP_CHECKMULTISIG)
                        || opcode == Instruction::Op(OP_CHECKMULTISIGVERIFY)
                }
                _ => false,
            })
    } else {
        false
    }
}

pub fn is_signed_witness(witness: &Witness) -> bool {
    if let Some(redeem_script_bytes) = witness.last() {
        let redeem_script = Script::from_bytes(redeem_script_bytes);
        redeem_script
            .instructions()
            .any(|instruction| match instruction {
                Ok(opcode) => {
                    opcode == Instruction::Op(OP_CHECKMULTISIG)
                        || opcode == Instruction::Op(OP_CHECKMULTISIGVERIFY)
                        || opcode == Instruction::Op(OP_CHECKSIG)
                        || opcode == Instruction::Op(OP_CHECKSIGVERIFY)
                }
                _ => false,
            })
    } else {
        false
    }
}

pub struct Checker {
    btccli: btcrpc::BtcCli,
}

impl Checker {
    pub fn new(btccli: btcrpc::BtcCli) -> Self {
        Checker { btccli }
    }

    pub fn check_input_sign(&self, input: &TxIn) -> bool {
        let prev_out = self
            .btccli
            .get_tx_out(&input.previous_output.txid, input.previous_output.vout)
            .unwrap();
        let signed = witness::check_input_signed(input, &prev_out);

        signed
    }

    pub fn check_sign(&self, tx: Transaction) -> Option<Vec<usize>> {
        let mut idxs = vec![];
        for (idx, input) in tx.input.iter().enumerate() {
            let prev_out = self
                .btccli
                .get_tx_out(&input.previous_output.txid, input.previous_output.vout)
                .unwrap();
            let signed = witness::check_input_signed(input, &prev_out);
            if !signed {
                idxs.push(idx);
            }
        }

        if idxs.len() > 0 {
            return Some(idxs);
        }
        None
    }
}
