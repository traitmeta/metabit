use super::*;

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

pub struct SignChecker {
    btccli: btcrpc::BtcCli,
}

impl SignChecker {
    pub fn new(btccli: btcrpc::BtcCli) -> Self {
        SignChecker { btccli }
    }

    pub fn check_input_sign(&self, input: &TxIn) -> bool {
        let prev_out = self
            .btccli
            .get_tx_out(&input.previous_output.txid, input.previous_output.vout);
        match prev_out {
            Ok(out) => witness::check_input_signed(input, Some(out)),
            Err(e) => {
                error!("get tx output from node failed : {}", e);
                false
            }
        }
    }

    pub fn check_sign(&self, tx: Transaction) -> Option<Vec<usize>> {
        let mut idxs = vec![];
        for (idx, input) in tx.input.iter().enumerate() {
            let prev_out = self
                .btccli
                .get_tx_out(&input.previous_output.txid, input.previous_output.vout)
                .unwrap();
            let signed = witness::check_input_signed(input, Some(prev_out));
            if !signed {
                idxs.push(idx);
            }
        }

        if !idxs.is_empty() {
            return Some(idxs);
        }
        None
    }

    pub fn check_sign_fast(&self, tx: &Transaction) -> Option<Vec<usize>> {
        let mut idxs = vec![];
        for (idx, input) in tx.input.iter().enumerate() {
            let signed = witness::check_input_signed(input, None);
            if !signed {
                idxs.push(idx);
            }
        }

        if !idxs.is_empty() {
            return Some(idxs);
        }
        None
    }
}
