use bitcoin::consensus::deserialize;
use datatypes::types;
use sender::unsign::UnsginSender;
use tracing::{debug, info};

use super::*;

pub struct SigHashNone {
    // subscriber: Arc<Socket>,
    sign_checker: SignChecker,
    unsgin_sender: UnsginSender,
}

impl SigHashNone {
    pub async fn new(cfg: &config::Config) -> Self {
        let btccli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
        let sign_checker = SignChecker::new(btccli);
        Self {
            sign_checker: sign_checker,
            unsgin_sender: UnsginSender::new(cfg),
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn handle_recv(&self, tx_data: Vec<u8>) -> Result<()> {
        if tx_data.is_empty() {
            return Ok(());
        }

        debug!("received from zmq : {:?}", tx_data);
        match deserialize::<Transaction>(&tx_data) {
            Ok(tx) => {
                debug!("received tx : {}", tx.compute_txid());
                let my_utxo = vec![];
                self.handle_tx_thread(&tx, &my_utxo);
            }
            Err(e) => {
                error!(
                    "Failed to deserialize transaction: received: {:?},{}",
                    tx_data, e
                );
            }
        }

        Ok(())
    }

    fn handle_tx_thread(&self, tx: &Transaction, my_utxo: &[types::Utxo]) {
        if tx.is_coinbase() {
            return;
        }

        let txid = tx.compute_txid();
        match self.sign_checker.check_sign_fast(tx) {
            Some(idxs) => {
                for idx in idxs.iter() {
                    info!("Received transaction hash: {}, idx : {}", txid, idx);
                    match self
                        .unsgin_sender
                        .send_unsigned_tx(tx, *idx as u32, my_utxo)
                    {
                        Ok(_) => {}
                        Err(e) => error!("send msg to channel failed. {}", e),
                    }
                }
            }
            None => {}
        }
    }
}


fn main() {
    // Replace with your own transaction hex
    let tx_hex = "0200000001...";  // Example transaction
    let tx_bytes = hex::decode(tx_hex).expect("Invalid hex");
    let tx: Transaction = deserialize(&tx_bytes).expect("Failed to decode transaction");

    for (i, input) in tx.input.iter().enumerate() {
        // If the scriptSig has a signature, extract the sighash type (P2PKH, P2SH)
        if let Some(signature) = extract_signature(&input.script_sig) {
            let sighash_type = get_sighash_type(&signature);
            if sighash_type == SigHashType::NONE {
                println!("Input {} is signed with SIGHASH_NONE (P2PKH/P2SH)", i);
            }
        }

        // If there is witness data (P2WPKH, P2WSH)
        if !input.witness.is_empty() {
            if let Some(signature) = input.witness.first() {
                let sighash_type = get_sighash_type(&signature);
                if sighash_type == SigHashType::NONE {
                    println!("Input {} is signed with SIGHASH_NONE (P2WPKH/P2WSH)", i);
                }
            }
        }

        // For Taproot (P2TR)
        if let Some(taproot_witness) = input.witness.first() {
            if is_taproot_signature(taproot_witness) {
                let sighash_type = get_sighash_type(taproot_witness);
                if sighash_type == SigHashType::NONE {
                    println!("Input {} is signed with SIGHASH_NONE (P2TR)", i);
                }
            }
        }
    }
}

// Extract the signature from a P2PKH or P2SH scriptSig
fn extract_signature(script_sig: &bitcoin::Script) -> Option<Vec<u8>> {
    if !script_sig.is_empty() {
        let data = script_sig.to_bytes();
        Some(data) // Assuming the first element is the signature
    } else {
        None
    }
}

// Get the SIGHASH type from the signature (last byte)
fn get_sighash_type(signature: &[u8]) -> SigHashType {
    SigHashType::from_u32(*signature.last().unwrap() as u32)
}

// Check if a witness is for Taproot
fn is_taproot_signature(witness: &[u8]) -> bool {
    witness.len() == 64 // Taproot Schnorr signatures are 64 bytes
}