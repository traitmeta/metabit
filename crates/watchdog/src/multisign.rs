use bitcoin::blockdata::opcodes::all::{
    OP_CHECKMULTISIG, OP_CHECKMULTISIGVERIFY, OP_CHECKSIG, OP_CHECKSIGVERIFY,
};
use bitcoin::script::Instruction;
use bitcoin::{Script, Witness};

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
