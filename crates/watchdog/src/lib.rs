pub mod btcrpc;
pub mod checker;
pub mod config;
pub mod lightning;
pub mod receiver;
pub mod repo;
pub mod sender;
pub mod syncer;

use anyhow::{anyhow, Result};
use bitcoin::blockdata::opcodes::all::{
    OP_CHECKMULTISIG, OP_CHECKMULTISIGVERIFY, OP_CHECKSIG, OP_CHECKSIGVERIFY,
};
use bitcoin::script::Instruction;
use bitcoin::{consensus::deserialize, Script, Transaction, TxIn, TxOut, Txid, Witness};
use bittx::witness;
use repo::Dao;
use std::{sync::Arc, time::Duration};
use tgbot::TgBot;
use tokio::{sync::broadcast::Receiver, time::sleep};
use tracing::{debug, error, info, warn};
use zmq::{Context, Socket};
