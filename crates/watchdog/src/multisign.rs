use bitcoin::blockdata::opcodes::all::{
    OP_CHECKMULTISIG, OP_CHECKMULTISIGVERIFY, OP_CHECKSIG, OP_CHECKSIGVERIFY,
};
use bitcoin::blockdata::script::Instruction;
use bitcoin::blockdata::script::Script;

pub fn is_multisig_witness(witness: &Vec<Vec<u8>>) -> bool {
    if let Some(redeem_script_bytes) = witness.last() {
        let redeem_script = Script::from(redeem_script_bytes.clone());
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

pub fn is_signed_witness(witness: &Vec<Vec<u8>>) -> bool {
    if let Some(redeem_script_bytes) = witness.last() {
        let redeem_script = Script::from(redeem_script_bytes.clone());
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
