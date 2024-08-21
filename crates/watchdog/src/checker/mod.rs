pub mod lightning;
pub mod sign;

use bitcoin::blockdata::opcodes::all::{
    OP_CHECKMULTISIG, OP_CHECKMULTISIGVERIFY, OP_CHECKSIG, OP_CHECKSIGVERIFY,
};
use bitcoin::script::Instruction;
use bitcoin::{Script, Transaction, TxIn, Txid, Witness};
use bittx::witness;

use crate::btcrpc;
