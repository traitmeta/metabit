use bitcoin::consensus::deserialize;
use datatypes::types;
use sender::unsign::UnsginSender;
use tracing::{debug, info};

use super::*;

pub struct UnsignedDog {
    sign_checker: SignChecker,
    unsgin_sender: UnsginSender,
}

impl UnsignedDog {
    pub async fn new(cfg: &config::Config) -> Self {
        let btccli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
        let sign_checker = SignChecker::new(btccli);
        Self {
            sign_checker: sign_checker,
            unsgin_sender: UnsginSender::new(cfg),
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn handle_recv(&self, tx_data: Vec<u8>, my_utxos: Vec<types::Utxo>) -> Result<()> {
        if tx_data.is_empty() {
            return Ok(());
        }

        debug!("received from zmq : {:?}", tx_data);
        match deserialize::<Transaction>(&tx_data) {
            Ok(tx) => {
                debug!("received tx : {}", tx.compute_txid());
                self.handle_tx_thread(&tx, &my_utxos).await;
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

    async fn handle_tx_thread(&self, tx: &Transaction, my_utxo: &[types::Utxo]) {
        if tx.is_coinbase() {
            return;
        }

        let txid = tx.compute_txid();
        if !self.sign_checker.check_sign_fast(tx) {
            info!("Received transaction hash: {}, idx : {}", txid, 0);
            match self.unsgin_sender.send_unsigned_tx(tx, 0, my_utxo).await {
                Ok(txid) => {
                    info!("sent transaction hash: {}", txid);
                }
                Err(e) => error!("send msg to channel failed. {}", e),
            }
        }
    }
}
